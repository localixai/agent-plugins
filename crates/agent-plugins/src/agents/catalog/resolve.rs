//! Port of `src/agents/catalog/resolve.ts`, switched.
//!
//! Which definition a name means (sub-agents.md, "Choose the subagent scope": "When multiple
//! subagents share the same name, Claude Code uses the one from the higher-priority location";
//! "the definition closest to the working directory" among nested project directories; a user or
//! project subagent named `Explore` overrides the built-in) and which subagents the settings switch
//! off ("Disable specific subagents": `Agent(name)` deny rules, or the `Agent` tool itself).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::builtin::FALLBACK_AGENT;
use super::scan::ScannedAgent;
use super::scopes::{BUILTIN_PRIORITY, agent_scope_rule};
use super::types::{CatalogAgent, CatalogTier, catalog_tier};
use crate::agents::tool_list::{AGENT_TOOL, is_agent_tool_name};
use crate::format::agent_record::AgentSource;
use crate::format::issues::{PluginIssue, warning};
use crate::format::listing::by_code_unit;
use crate::js::{JsObject, JsValue};
use crate::rules::evaluate::evaluate_rules;
use crate::rules::tool_pattern::tool_name_matches;
use crate::rules::types::{
    MatchContext, PathContext, PermissionBehavior, PermissionRule, Platform, PreparedCall, ToolCall,
};

/// Every source of a session's subagents.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSources {
    /// The built-ins.
    #[serde(default)]
    pub builtin: Option<Vec<CatalogAgent>>,
    /// The `--agents` flag's agents.
    #[serde(default)]
    pub flag: Option<Vec<CatalogAgent>>,
    /// The files of the managed, project, user and added folders, as `scanAgents` read them.
    #[serde(default)]
    pub scanned: Option<Vec<ScannedAgent>>,
    /// The agents of the enabled plugins, under their scoped names.
    #[serde(default)]
    pub plugin: Option<Vec<CatalogAgent>>,
    /// The permission rules of the session; only deny rules matter here (settings and
    /// `--disallowedTools`).
    #[serde(default)]
    pub denied: Option<Vec<PermissionRule>>,
}

/// A definition that lost its name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shadowed {
    /// The definition.
    pub agent: CatalogAgent,
    /// The definition that took the name.
    pub by: CatalogAgent,
}

/// The subagents a session can run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentCatalog {
    /// The subagents a session can run, highest priority first.
    pub agents: Vec<CatalogAgent>,
    /// Removed by a deny rule: the built-in or custom subagents that were defined.
    pub disabled: Vec<CatalogAgent>,
    /// Definitions that lost their name to a higher-priority one.
    pub shadowed: Vec<Shadowed>,
    /// Two files of one agents directory with one name.
    pub issues: Vec<PluginIssue>,
    /// False when a deny rule names the `Agent` tool itself: no subagent can be spawned.
    pub delegation: bool,
}

/// The Priority column of the table; a built-in comes after every source of it.
const fn priority(tier: CatalogTier) -> u32 {
    match tier {
        CatalogTier::Managed => agent_scope_rule(AgentSource::Managed).priority,
        CatalogTier::Flag => agent_scope_rule(AgentSource::Flag).priority,
        CatalogTier::Project => agent_scope_rule(AgentSource::Project).priority,
        CatalogTier::User => agent_scope_rule(AgentSource::User).priority,
        CatalogTier::Plugin => agent_scope_rule(AgentSource::Plugin).priority,
        CatalogTier::Builtin => BUILTIN_PRIORITY,
    }
}

struct Candidate<'a> {
    agent: &'a CatalogAgent,
    order: u32,
    canonical: Option<&'a str>,
}

fn compare(a: &Candidate<'_>, b: &Candidate<'_>) -> std::cmp::Ordering {
    priority(catalog_tier(&a.agent.source))
        .cmp(&priority(catalog_tier(&b.agent.source)))
        .then(a.order.cmp(&b.order))
        .then_with(|| {
            by_code_unit(
                a.agent.path.as_deref().unwrap_or_default(),
                b.agent.path.as_deref().unwrap_or_default(),
            )
        })
}

fn plain(agents: Option<&Vec<CatalogAgent>>) -> impl Iterator<Item = Candidate<'_>> {
    agents.into_iter().flatten().map(|agent| Candidate {
        agent,
        order: 0,
        canonical: None,
    })
}

fn candidates_of(sources: &AgentSources) -> Vec<Candidate<'_>> {
    let scanned = sources.scanned.iter().flatten().map(|entry| Candidate {
        agent: &entry.agent,
        order: entry.order,
        canonical: entry.canonical.as_deref(),
    });
    let mut all: Vec<Candidate<'_>> = plain(sources.builtin.as_ref())
        .chain(plain(sources.flag.as_ref()))
        .chain(scanned)
        .chain(plain(sources.plugin.as_ref()))
        .collect();
    all.sort_by(compare);
    all
}

/// Two names for one file (a symbolic link) are one agent: the first, by rank, stands.
fn without_aliases(candidates: Vec<Candidate<'_>>) -> Vec<Candidate<'_>> {
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|candidate| {
            let Some(canonical) = candidate.canonical else {
                return true;
            };
            let tier = catalog_tier(&candidate.agent.source);
            seen.insert(format!("{tier:?}|{canonical}"))
        })
        .collect()
}

/// The rules cannot see paths in an agent rule; this context only satisfies the matcher's
/// signature.
fn names_only() -> MatchContext {
    MatchContext {
        paths: PathContext {
            cwd: "/".to_owned(),
            home: "/".to_owned(),
            primary_working_directory: "/".to_owned(),
            additional_directories: Vec::new(),
            platform: Platform::Posix,
            real_directories: None,
        },
        safe_env_vars: None,
        depth_of: None,
    }
}

/// "Existing `Task(...)` references in settings and agent definitions still work as aliases": a
/// rule written for `Task` is read as the same rule for `Agent`.
fn as_agent_rule(rule: &PermissionRule) -> PermissionRule {
    let mut rule = rule.clone();
    if is_agent_tool_name(&rule.value.tool_name) {
        AGENT_TOOL.clone_into(&mut rule.value.tool_name);
    }
    rule
}

/// A bare `Agent` deny rule: the tool itself, which "prevents Claude from delegating to any
/// subagent".
fn denies_tool(rules: &[PermissionRule]) -> bool {
    rules
        .iter()
        .any(|rule| rule.specifier.is_none() && tool_name_matches(&as_agent_rule(rule), AGENT_TOOL))
}

fn is_disabled(rules: &[PermissionRule], name: &str) -> Result<bool, String> {
    let input = JsObject::from([("subagent_type".to_owned(), JsValue::from(name))]);
    let call = PreparedCall {
        call: ToolCall {
            tool_name: AGENT_TOOL.to_owned(),
            input: JsValue::from(input),
        },
        resolved_paths: None,
        skill_names: None,
        allowed_domains: None,
    };
    let verdict = evaluate_rules(rules, &call, &names_only())?;
    Ok(verdict.behavior == Some(PermissionBehavior::Deny))
}

#[derive(Default)]
struct Grouped {
    winners: Vec<CatalogAgent>,
    shadowed: Vec<Shadowed>,
    issues: Vec<PluginIssue>,
}

fn same_folder(a: &Candidate<'_>, b: &Candidate<'_>) -> bool {
    catalog_tier(&a.agent.source) == catalog_tier(&b.agent.source) && a.order == b.order
}

/// Sorted candidates, one winner per name; the rest are shadowed (and a same-folder twin is a
/// warning).
fn group_by_name(candidates: &[Candidate<'_>]) -> Grouped {
    let mut first: HashMap<&str, &Candidate<'_>> = HashMap::new();
    let mut grouped = Grouped::default();
    for candidate in candidates {
        let Some(holder) = first.get(candidate.agent.name.as_str()).copied() else {
            first.insert(&candidate.agent.name, candidate);
            grouped.winners.push(candidate.agent.clone());
            continue;
        };
        grouped.shadowed.push(Shadowed {
            agent: candidate.agent.clone(),
            by: holder.agent.clone(),
        });
        if same_folder(holder, candidate) {
            let name = &candidate.agent.name;
            let other = holder.agent.path.as_deref().unwrap_or("another file");
            grouped.issues.push(warning(
                candidate.agent.path.as_deref().unwrap_or(name),
                // Judgment (spec/judgments.md): the docs leave the choice to "filesystem read
                // order"; the kit takes the first path in sorted order so the answer does not
                // change between runs.
                &format!(
                    "Agent \"{name}\" is defined here and in {other}, in the same directory; only that one loads"
                ),
            ));
        }
    }
    grouped
}

/// The agents of every source, one definition per name, minus what a deny rule switched off.
///
/// # Errors
///
/// When a deny rule cannot be matched (a pattern that does not compile).
pub fn resolve_agents(sources: &AgentSources) -> Result<AgentCatalog, String> {
    let rules: Vec<PermissionRule> = sources
        .denied
        .iter()
        .flatten()
        .filter(|rule| rule.behavior == PermissionBehavior::Deny)
        .cloned()
        .collect();
    let grouped = group_by_name(&without_aliases(candidates_of(sources)));
    let delegation = !denies_tool(&rules);
    let agent_rules: Vec<PermissionRule> = rules.iter().map(as_agent_rule).collect();
    let mut agents = Vec::new();
    let mut disabled = Vec::new();
    for agent in grouped.winners {
        if !delegation || is_disabled(&agent_rules, &agent.name)? {
            disabled.push(agent);
        } else {
            agents.push(agent);
        }
    }
    Ok(AgentCatalog {
        agents,
        disabled,
        shadowed: grouped.shadowed,
        issues: grouped.issues,
        delegation,
    })
}

/// The subagent an Agent tool call runs, or why there is none.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SubagentTypeOutcome {
    /// The subagent.
    Found {
        /// Always `true`.
        ok: bool,
        /// The subagent, boxed: a refusal is a line of text.
        agent: Box<CatalogAgent>,
    },
    /// Why there is none.
    Refused {
        /// Always `false`.
        ok: bool,
        /// What the call answers.
        message: String,
    },
}

fn listing(catalog: &AgentCatalog) -> String {
    let mut names: Vec<&str> = catalog
        .agents
        .iter()
        .map(|agent| agent.name.as_str())
        .collect();
    names.sort_by(|a, b| by_code_unit(a, b));
    names.join(", ")
}

/// The subagent an Agent tool call runs (sub-agents.md, "Built-in subagents": "An Agent tool call
/// that omits `subagent_type` fails with `subagent_type is required` when the session has no
/// `general-purpose` subagent to fall back on"; errors.md, "subagent_type is required"). The
/// message lists the subagents the session has so Claude can retry with one of them. Judgment
/// (spec/judgments.md): the list is alphabetical, and a type the session does not have, or one a
/// deny rule switched off, gets the same list under a message of the kit's own.
#[must_use]
pub fn resolve_subagent_type(
    catalog: &AgentCatalog,
    subagent_type: Option<&str>,
) -> SubagentTypeOutcome {
    if !catalog.delegation {
        return SubagentTypeOutcome::Refused {
            ok: false,
            message: "The Agent tool is not available".to_owned(),
        };
    }
    let wanted = subagent_type.unwrap_or(FALLBACK_AGENT);
    if let Some(agent) = catalog.agents.iter().find(|each| each.name == wanted) {
        return SubagentTypeOutcome::Found {
            ok: true,
            agent: Box::new(agent.clone()),
        };
    }
    let available = format!("Available agents: {}", listing(catalog));
    let message = match subagent_type {
        None => format!(
            "subagent_type is required: the general-purpose agent is not available in this session. {available}"
        ),
        Some(subagent_type) => format!("Agent type '{subagent_type}' not found. {available}"),
    };
    SubagentTypeOutcome::Refused { ok: false, message }
}

crate::registry::exports! {
    "agents/catalog/resolve" {
        "resolveAgents" => fallible |sources: Option<AgentSources>|
            resolve_agents(&sources.unwrap_or_default());
        "resolveSubagentType" => ok |catalog: AgentCatalog, subagent_type: Option<String>|
            resolve_subagent_type(&catalog, subagent_type.as_deref());
    }
}
