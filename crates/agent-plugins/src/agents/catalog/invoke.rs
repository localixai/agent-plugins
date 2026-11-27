//! Port of `src/agents/catalog/invoke.ts`, switched.
//!
//! Naming a subagent (sub-agents.md, "Invoke subagents explicitly"). Three patterns "escalate from a
//! one-off suggestion to a session-wide default": a name in the prompt (Claude decides, nothing to
//! parse), an @-mention (the subagent runs for that task) and the whole session as the subagent
//! (`--agent`, the `agent` setting). Pure over the catalog; the typeahead and the session start are the
//! host's.
//!
//! `AgentCatalog` is `agents/catalog/resolve.ts`'s, which stays TypeScript: here it is the two lists
//! this module reads.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::mention_scan::{matches_of, picked_at, typed_at};
use super::types::CatalogAgent;
use crate::format::listing::by_code_unit;

/// The subagents a session can run, and the ones a deny rule switched off.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AgentCatalog {
    /// The subagents a session can run, highest priority first.
    pub agents: Vec<CatalogAgent>,
    /// Removed by a deny rule: the built-in or custom subagents that were defined.
    pub disabled: Vec<CatalogAgent>,
}

/// The typeahead's label: `code-reviewer (agent)`; "under their scoped name" for a plugin subagent.
fn label_of(agent: &CatalogAgent) -> String {
    format!("{} (agent)", agent.name)
}

/// One subagent offered after `@`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TypeaheadEntry {
    /// What the list shows.
    pub label: String,
    /// What picking it writes into the prompt: `@"code-reviewer (agent)"`.
    pub insert: String,
    /// The subagent.
    pub agent: CatalogAgent,
}

/// The subagents to offer after `@`. Judgment (spec/judgments.md): a subagent a deny rule switched off is
/// not offered (nor mentioned); the docs say only that the rule stops Claude using it.
#[must_use]
pub fn agent_typeahead(catalog: &AgentCatalog) -> Vec<TypeaheadEntry> {
    catalog
        .agents
        .iter()
        .map(|agent| TypeaheadEntry {
            label: label_of(agent),
            insert: format!("@\"{}\"", label_of(agent)),
            agent: agent.clone(),
        })
        .collect()
}

/// One subagent a message mentions.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgentMention {
    /// The subagent.
    pub agent: CatalogAgent,
    /// The mention as written, `@agent-test-runner`.
    pub text: String,
}

/// What a message mentions.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MentionsFound {
    /// Each subagent once, in the order first mentioned.
    pub mentions: Vec<AgentMention>,
    /// The message Claude receives: "Your full message still goes to Claude, which writes the subagent's
    /// task prompt ... The @-mention controls which subagent Claude invokes, not what prompt it receives."
    pub message: String,
}

struct Found<'a> {
    index: usize,
    text: String,
    agent: &'a CatalogAgent,
}

fn by_name(catalog: &AgentCatalog) -> HashMap<&str, &CatalogAgent> {
    catalog
        .agents
        .iter()
        .map(|agent| (agent.name.as_str(), agent))
        .collect()
}

/// The longest start of the typed token that names a subagent: `test-runner,` is `test-runner`.
fn longest_known<'a>(
    token: &str,
    known: &HashMap<&str, &'a CatalogAgent>,
) -> Option<(&'a CatalogAgent, String)> {
    let ends = token
        .char_indices()
        .map(|(index, character)| index + character.len_utf8());
    ends.rev().find_map(|end| {
        let prefix = token.get(..end)?;
        known.get(prefix).map(|agent| (*agent, prefix.to_owned()))
    })
}

fn picked_mentions<'a>(message: &str, known: &HashMap<&str, &'a CatalogAgent>) -> Vec<Found<'a>> {
    matches_of(message, picked_at)
        .into_iter()
        .filter_map(|(index, length, name)| {
            let agent = known.get(name.as_str())?;
            Some(Found {
                index,
                text: message.get(index..index + length)?.to_owned(),
                agent,
            })
        })
        .collect()
}

fn typed_mentions<'a>(message: &str, known: &HashMap<&str, &'a CatalogAgent>) -> Vec<Found<'a>> {
    matches_of(message, typed_at)
        .into_iter()
        .filter_map(|(index, _, token)| {
            let (agent, prefix) = longest_known(&token, known)?;
            Some(Found {
                index,
                text: format!("@agent-{prefix}"),
                agent,
            })
        })
        .collect()
}

/// The subagents a message mentions, from both spellings ("You can also type the mention manually without
/// using the picker: `@agent-<name>` for local subagents, or `@agent-` followed by the scoped name for
/// plugin subagents"). A name that is no subagent of the catalog selects nothing. Natural language is
/// not a mention: "Use the test-runner subagent" leaves the choice to Claude.
#[must_use]
pub fn agent_mentions(message: &str, catalog: &AgentCatalog) -> MentionsFound {
    let known = by_name(catalog);
    let mut found = picked_mentions(message, &known);
    found.extend(typed_mentions(message, &known));
    found.sort_by_key(|each| each.index);
    let mut seen: Vec<&str> = Vec::new();
    let mut mentions = Vec::new();
    for Found { agent, text, .. } in found {
        if seen.contains(&agent.name.as_str()) {
            continue;
        }
        seen.push(&agent.name);
        mentions.push(AgentMention {
            agent: agent.clone(),
            text,
        });
    }
    MentionsFound {
        mentions,
        message: message.to_owned(),
    }
}

/// Where the choice of the session agent came from: `--agent` ("The CLI flag overrides the setting"),
/// the `agent` key of a settings source, or a plugin's default `agent` (plugins/components.md, "Default
/// settings": the lowest layer).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionAgentSource {
    /// `--agent`.
    Flag,
    /// The `agent` key of a settings source.
    Setting,
    /// A plugin's default.
    Plugin,
}

/// The `agent` setting as the settings resolved it: the name, and the plugin when it is a plugin default.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AgentSetting {
    /// The agent's name.
    pub name: String,
    /// The id of the plugin whose `settings.json` (or manifest `settings`) set it; absent for a settings source.
    #[serde(default)]
    pub plugin: Option<String>,
}

/// The agent a session runs as, or why none can be.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SessionAgentChoice {
    /// The choice resolved.
    Chosen {
        /// Always true.
        ok: bool,
        /// The agent, or none when nothing was asked.
        #[serde(skip_serializing_if = "Option::is_none")]
        agent: Option<Box<CatalogAgent>>,
        /// Where the choice came from.
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<SessionAgentSource>,
        /// With `from: 'plugin'`: the id of the plugin whose default chose it.
        #[serde(skip_serializing_if = "Option::is_none")]
        plugin: Option<String>,
    },
    /// The name means no single agent.
    Refused {
        /// Always false.
        ok: bool,
        /// Why.
        message: String,
        /// The agents the name could mean, in code-unit order of their scoped names: two or more for a
        /// bare name several plugins provide, none for a name no agent has.
        candidates: Vec<CatalogAgent>,
    },
}

fn chosen(agent: Option<CatalogAgent>) -> SessionAgentChoice {
    SessionAgentChoice::Chosen {
        ok: true,
        agent: agent.map(Box::new),
        from: None,
        plugin: None,
    }
}

/// Every subagent the person may choose: a deny rule stops delegation to one, not choosing it as the session's.
fn choosable(catalog: &AgentCatalog) -> Vec<&CatalogAgent> {
    catalog.agents.iter().chain(&catalog.disabled).collect()
}

fn lookup(catalog: &AgentCatalog, wanted: &str) -> SessionAgentChoice {
    let all = choosable(catalog);
    if let Some(exact) = all.iter().find(|agent| agent.name == wanted) {
        return chosen(Some((*exact).clone()));
    }
    // "For a plugin-provided subagent, you can pass only the agent name and Claude Code finds it".
    let bare: Vec<&CatalogAgent> = all
        .into_iter()
        .filter(|agent| agent.own_name == wanted)
        .collect();
    if let [only] = bare.as_slice() {
        return chosen(Some((*only).clone()));
    }
    if bare.len() > 1 {
        let mut candidates: Vec<CatalogAgent> = bare.into_iter().cloned().collect();
        candidates.sort_by(|first, second| by_code_unit(&first.name, &second.name));
        let scoped: Vec<&str> = candidates.iter().map(|agent| agent.name.as_str()).collect();
        return SessionAgentChoice::Refused {
            ok: false,
            message: format!(
                "Agent \"{wanted}\" is provided by several plugins; pass the scoped name: {}",
                scoped.join(", ")
            ),
            candidates,
        };
    }
    SessionAgentChoice::Refused {
        ok: false,
        message: format!("No subagent named \"{wanted}\""),
        candidates: Vec::new(),
    }
}

/// What the person chose: `--agent` and the `agent` setting.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct SessionChoice {
    /// `--agent <name>`.
    #[serde(default)]
    pub flag: Option<String>,
    /// The `agent` setting.
    #[serde(default)]
    pub setting: Option<AgentSetting>,
}

/// The subagent a session runs as: `--agent <name>` over the `agent` setting ("`--agent` takes
/// precedence over this key for one session"), the setting as `valueWithPluginDefaults` resolved it (a
/// settings source over the plugins' defaults). Oracle 2.1.322 (spec/judgments.md): a plugin's default
/// names an agent the way the setting does, so a user agent of that exact name wins over the plugin's own.
/// Judgment (spec/judgments.md): the docs name no failure for an unknown name, so the message is the
/// kit's, and a deny rule `Agent(name)` does not stop the person choosing it as the main thread.
#[must_use]
pub fn select_session_agent(catalog: &AgentCatalog, choice: &SessionChoice) -> SessionAgentChoice {
    if let Some(flag) = &choice.flag {
        return match lookup(catalog, flag) {
            SessionAgentChoice::Chosen {
                ok, agent, plugin, ..
            } => SessionAgentChoice::Chosen {
                ok,
                agent,
                from: Some(SessionAgentSource::Flag),
                plugin,
            },
            refused @ SessionAgentChoice::Refused { .. } => refused,
        };
    }
    let Some(setting) = &choice.setting else {
        return chosen(None);
    };
    match lookup(catalog, &setting.name) {
        SessionAgentChoice::Chosen { ok, agent, .. } => SessionAgentChoice::Chosen {
            ok,
            agent,
            from: Some(if setting.plugin.is_none() {
                SessionAgentSource::Setting
            } else {
                SessionAgentSource::Plugin
            }),
            plugin: setting.plugin.clone(),
        },
        refused @ SessionAgentChoice::Refused { .. } => refused,
    }
}

/// The startup header's mark: "The agent name appears as `@<name>`".
#[must_use]
pub fn session_agent_label(agent: &CatalogAgent) -> String {
    format!("@{}", agent.name)
}

/// The built-in agent a session falls back to: its absence changes nothing, so it never warns.
const DEFAULT_AGENT: &str = "claude";

/// The agent of a resumed session.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResumedSessionAgent {
    /// The agent, or none when it is gone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<CatalogAgent>,
    /// Shown in the resumed conversation when the agent is gone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// The agent of a resumed session (sub-agents.md: "the choice persists when you resume the session";
/// errors.md, "Session agent no longer available"). A missing one lets the session go on with the
/// default tools and a warning that names it and the directories searched.
#[must_use]
pub fn resume_session_agent(
    catalog: &AgentCatalog,
    name: &str,
    searched: &[String],
) -> ResumedSessionAgent {
    if let SessionAgentChoice::Chosen { agent, .. } = lookup(catalog, name) {
        return ResumedSessionAgent {
            agent: agent.map(|found| *found),
            warning: None,
        };
    }
    if name == DEFAULT_AGENT {
        return ResumedSessionAgent {
            agent: None,
            warning: None,
        };
    }
    ResumedSessionAgent {
        agent: None,
        warning: Some(format!(
            "This session was running agent '{name}', which is no longer available (no agent by that name in {}). Continuing with the default tools and system prompt — the agent's tool restrictions no longer apply. To restore it, re-create the agent, or resume with an explicit --agent <name>.",
            searched.join(", ")
        )),
    }
}

crate::registry::exports! {
    "agents/catalog/invoke" {
        "agentTypeahead" => ok |catalog: AgentCatalog| agent_typeahead(&catalog);
        "agentMentions" => ok |message: String, catalog: AgentCatalog| agent_mentions(&message, &catalog);
        "selectSessionAgent" => ok |catalog: AgentCatalog, choice: SessionChoice| select_session_agent(&catalog, &choice);
        "sessionAgentLabel" => ok |agent: CatalogAgent| session_agent_label(&agent);
        "resumeSessionAgent" => ok |catalog: AgentCatalog, name: String, searched: Vec<String>| resume_session_agent(&catalog, &name, &searched);
    }
}
