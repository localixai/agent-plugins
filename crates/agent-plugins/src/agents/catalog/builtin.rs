//! Port of `src/agents/catalog/builtin.ts`, switched.
//!
//! The built-in subagents (sub-agents.md, "Built-in subagents"): the three the page gives a tab each and
//! the "Other" table. A registry typed over the names, so a built-in added to the list fails the
//! typecheck until it says what it is. The docs publish no prompt (a built-in's is Claude Code's own),
//! so `prompt` is empty; a description is the page's own sentence for it (spec/judgments.md).

use serde::Deserialize;

use super::types::{CatalogAgent, CatalogSource, StartContext};
use crate::agents::model_value::ModelAlias;
use crate::format::agent_record::AgentExtras;
use crate::js::JsValue;

/// The built-ins, in the page's order.
pub const BUILTIN_AGENT_NAMES: [&str; 6] = [
    "Explore",
    "Plan",
    "general-purpose",
    "claude",
    "statusline-setup",
    "claude-code-guide",
];

/// The subagent a call without `subagent_type` runs: "a `general-purpose` subagent to fall back on".
pub const FALLBACK_AGENT: &str = "general-purpose";

struct BuiltinSpec {
    description: &'static str,
    /// "Tools: read-only tools; Write and Edit are denied". Absent: every tool available to subagents.
    denied: Option<&'static [&'static str]>,
    /// Its own model; absent, the model order of "Choose a model" decides.
    model: Option<ModelAlias>,
    starts_with: StartContext,
    /// The two `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS` removes.
    explore_or_plan: bool,
}

const FULL: StartContext = StartContext {
    claude_md: true,
    git_status: true,
};
/// "Explore and Plan start without your CLAUDE.md files and the git status snapshot".
const LEAN: StartContext = StartContext {
    claude_md: false,
    git_status: false,
};
const READ_ONLY: &[&str] = &["Write", "Edit"];

/// The spec of a name of `BUILTIN_AGENT_NAMES`.
fn builtin_spec(name: &str) -> Option<BuiltinSpec> {
    match name {
        "Explore" => Some(BuiltinSpec {
            description: "A fast, read-only agent optimized for searching and analyzing codebases.",
            denied: Some(READ_ONLY),
            model: None,
            starts_with: LEAN,
            explore_or_plan: true,
        }),
        "Plan" => Some(BuiltinSpec {
            description: "A research agent used during plan mode to gather context before presenting a plan.",
            denied: Some(READ_ONLY),
            model: None,
            starts_with: LEAN,
            explore_or_plan: true,
        }),
        "general-purpose" => Some(BuiltinSpec {
            description: "A capable agent for complex, multi-step tasks that require both exploration and action.",
            denied: None,
            model: None,
            starts_with: FULL,
            explore_or_plan: false,
        }),
        "claude" => Some(BuiltinSpec {
            description: "A catch-all for a task that doesn't fit a more specialized agent, with every tool available to subagents.",
            denied: None,
            model: None,
            starts_with: FULL,
            explore_or_plan: false,
        }),
        "statusline-setup" => Some(BuiltinSpec {
            description: "Configures your status line when you run /statusline.",
            denied: None,
            model: Some(ModelAlias::Sonnet),
            starts_with: FULL,
            explore_or_plan: false,
        }),
        "claude-code-guide" => Some(BuiltinSpec {
            description: "Answers questions about Claude Code features.",
            denied: None,
            model: Some(ModelAlias::Haiku),
            starts_with: FULL,
            explore_or_plan: false,
        }),
        _ => None,
    }
}

/// What switches the built-ins off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinPolicy {
    /// False with `-p` or through the Agent SDK.
    pub interactive: bool,
    /// `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS=1`.
    pub disable_explore_plan: bool,
    /// `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS=1`; it acts in non-interactive mode only.
    pub disable_all: bool,
}

/// Whether the built-in is registered under the policy: "registered by default in interactive sessions".
const fn registered(spec: &BuiltinSpec, policy: BuiltinPolicy) -> bool {
    if policy.disable_all && !policy.interactive {
        return false;
    }
    !(policy.disable_explore_plan && spec.explore_or_plan)
}

fn catalog_entry(name: &str, spec: &BuiltinSpec) -> CatalogAgent {
    let mut definition = vec![
        ("description", JsValue::from(spec.description)),
        ("prompt", JsValue::from("")),
    ];
    if let Some(denied) = spec.denied {
        definition.push((
            "disallowedTools",
            JsValue::from(
                denied
                    .iter()
                    .map(|tool| JsValue::from(*tool))
                    .collect::<Vec<_>>(),
            ),
        ));
    }
    if let Some(model) = spec.model {
        definition.push(("model", JsValue::from(model.as_str())));
    }
    CatalogAgent {
        name: name.to_owned(),
        own_name: name.to_owned(),
        source: CatalogSource::Builtin,
        definition: JsValue::from(
            definition
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect::<crate::js::JsObject>(),
        ),
        extras: AgentExtras::default(),
        hooks: None,
        path: None,
        starts_with: spec.starts_with,
    }
}

/// The built-in subagents a session registers, in the page's order.
#[must_use]
pub fn builtin_agents(policy: BuiltinPolicy) -> Vec<CatalogAgent> {
    BUILTIN_AGENT_NAMES
        .into_iter()
        .filter_map(|name| builtin_spec(name).map(|spec| (name, spec)))
        .filter(|(_, spec)| registered(spec, policy))
        .map(|(name, spec)| catalog_entry(name, &spec))
        .collect()
}

crate::registry::exports! {
    "agents/catalog/builtin" {
        "BUILTIN_AGENT_NAMES" => ok || BUILTIN_AGENT_NAMES;
        "builtinAgents" => ok |policy: BuiltinPolicy| builtin_agents(policy);
    }
}
