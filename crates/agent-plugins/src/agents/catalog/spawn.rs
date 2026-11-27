//! Port of `src/agents/catalog/spawn.ts`, switched.
//!
//! Who may spawn whom (sub-agents.md, "Restrict which subagents can be spawned", "Let subagents
//! spawn their own subagents" and "Subagent names"). The tool lists decide whether an agent has the
//! Agent tool and which types it may name (`resolveSubagentTools`); this applies that answer to the
//! catalog, counts the layers and decides what a `name` on the call does.

use serde::{Deserialize, Serialize};

use super::builtin::FALLBACK_AGENT;
use super::resolve::{AgentCatalog, SubagentTypeOutcome, resolve_subagent_type};
use crate::agents::tool_resolution::{SpawnTypes, SpawnWord};
use crate::format::env_flag::is_truthy_env;
use crate::js::trim;

/// Whether the allowlist lets the type through: exact, as an `Agent(Name)` rule compares
/// (spec/judgments.md).
fn spawn_allows(spawn: &SpawnTypes, kind: &str) -> bool {
    match spawn {
        SpawnTypes::Word(SpawnWord::Any) => true,
        SpawnTypes::Word(SpawnWord::None) => false,
        SpawnTypes::Types(types) => types.iter().any(|each| each == kind),
    }
}

/// The catalog an agent sees through its Agent tool: "This is an allowlist: only the `worker` and
/// `researcher` subagents can be spawned. If the agent tries to spawn any other type, the request
/// fails and the agent sees only the allowed types in its prompt." A built-in outside the list is
/// outside it; a name nothing defines is just not spawnable.
#[must_use]
pub fn spawnable_agents(catalog: &AgentCatalog, spawn: &SpawnTypes) -> AgentCatalog {
    if *spawn == SpawnTypes::Word(SpawnWord::Any) {
        return catalog.clone();
    }
    AgentCatalog {
        agents: catalog
            .agents
            .iter()
            .filter(|agent| spawn_allows(spawn, &agent.name))
            .cloned()
            .collect(),
        delegation: catalog.delegation && *spawn != SpawnTypes::Word(SpawnWord::None),
        ..catalog.clone()
    }
}

/// Why an Agent call may not spawn `subagent_type` under the caller's `spawnTypes`
/// (`resolveSubagentTools`), or `None` when the allowlist lets it through: "the request fails and
/// the agent sees only the allowed types", so the message is `resolveSubagentType`'s over
/// `spawnableAgents`. Whether the type exists is `resolveSubagentType`'s question, not this one's.
/// Judgment (spec/judgments.md): a call without `subagent_type` is `general-purpose` and a fork is
/// the type `fork` (the SDK's `AgentInput`), so a list that names neither refuses them.
#[must_use]
pub fn spawn_refusal(
    catalog: &AgentCatalog,
    spawn: &SpawnTypes,
    subagent_type: Option<&str>,
) -> Option<String> {
    if spawn_allows(spawn, subagent_type.unwrap_or(FALLBACK_AGENT)) {
        return None;
    }
    match resolve_subagent_type(&spawnable_agents(catalog, spawn), subagent_type) {
        SubagentTypeOutcome::Found { .. } => None,
        SubagentTypeOutcome::Refused { message, .. } => Some(message),
    }
}

/// "up to three layers below the main conversation".
pub const DEFAULT_SPAWN_DEPTH: u32 = 3;

/// `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`: "the number of subagent layers you want below your main
/// conversation". Judgment (spec/judgments.md): the docs name no invalid value, so anything but a
/// positive whole number leaves the default.
#[must_use]
pub fn spawn_depth_limit(value: Option<&str>) -> f64 {
    let text = value.map(trim).unwrap_or_default();
    let digits = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let number = if digits { crate::js::number(text) } else { 0.0 };
    if number > 0.0 {
        number
    } else {
        f64::from(DEFAULT_SPAWN_DEPTH)
    }
}

/// What the depth limit does to an agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Nesting {
    /// Whether `Agent` is among the agent's tools.
    pub agent_tool_listed: bool,
    /// Whether a call of it spawns; a fork at the limit lists the tool, which "returns an error
    /// instead".
    pub spawns: bool,
}

/// Where an agent is.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct NestingInput {
    /// Layers below the main conversation.
    pub layer: f64,
    /// The depth limit.
    pub limit: f64,
    /// The agent is a fork.
    pub fork: bool,
}

/// What the depth limit does to an agent `layer` layers below the main conversation (0 is the main
/// conversation, 1 a subagent it spawned). "At the depth limit, Claude Code withholds the `Agent`
/// tool from every subagent except a fork"; "With this value \[2\], your subagents can delegate to a
/// second layer of their own, and that second layer can't delegate further. Set `1` to turn
/// nesting off."
#[must_use]
pub fn nesting_at(input: NestingInput) -> Nesting {
    let at_limit = input.layer >= input.limit;
    Nesting {
        agent_tool_listed: input.fork || !at_limit,
        spawns: !at_limit,
    }
}

/// What `namedSubagent` reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's JSON shape: one fact per flag"
)]
pub struct NamedSubagentInput {
    /// The session started with agent teams enabled.
    pub agent_teams: bool,
    /// The session is interactive.
    pub interactive: bool,
    /// The `name` parameter of the Agent call.
    #[serde(default)]
    pub name: Option<String>,
    /// The call spawns a fork.
    pub fork: bool,
    /// The call passes `isolation` itself; the subagent's frontmatter does not count.
    pub isolation_on_call: bool,
    /// The main conversation makes the call, not another subagent.
    pub from_main_conversation: bool,
}

/// How a named subagent launches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchesAs {
    /// A subagent.
    Subagent,
    /// A teammate.
    Teammate,
}

/// What a `name` on the call does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedSubagent {
    /// Claude can message or resume it by name after it finishes.
    pub addressable: bool,
    /// What it launches as.
    pub launches_as: LaunchesAs,
}

/// What a `name` on the Agent call does (sub-agents.md, "Subagent names"): with agent teams
/// enabled it makes the subagent addressable, and "in an interactive session ... a subagent that
/// Claude spawns from the main conversation with a `name` launches as a teammate instead, unless
/// the call is a fork or passes `isolation` on the call itself". Judgment (spec/judgments.md):
/// without agent teams the `name` parameter is not part of the tool, so it names nothing.
#[must_use]
pub fn named_subagent(input: &NamedSubagentInput) -> NamedSubagent {
    let named = input.agent_teams && input.name.is_some();
    let teammate = named
        && input.interactive
        && input.from_main_conversation
        && !input.fork
        && !input.isolation_on_call;
    NamedSubagent {
        addressable: named,
        launches_as: if teammate {
            LaunchesAs::Teammate
        } else {
            LaunchesAs::Subagent
        },
    }
}

/// Whether agent teams are on: `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` set to an on spelling, in
/// the environment or a settings `env` (the host passes the merged value); off by default
/// (env-vars.md; spec §5.12.5). The value `NamedSubagentInput.agentTeams` takes.
#[must_use]
pub fn agent_teams_on(value: Option<&str>) -> bool {
    is_truthy_env(value)
}

crate::registry::exports! {
    "agents/catalog/spawn" {
        "spawnableAgents" => ok |catalog: AgentCatalog, spawn: SpawnTypes| spawnable_agents(&catalog, &spawn);
        "spawnRefusal" => ok |catalog: AgentCatalog, spawn: SpawnTypes, subagent_type: Option<String>|
            spawn_refusal(&catalog, &spawn, subagent_type.as_deref());
        "DEFAULT_SPAWN_DEPTH" => ok || DEFAULT_SPAWN_DEPTH;
        "spawnDepthLimit" => ok |value: Option<String>| spawn_depth_limit(value.as_deref());
        "nestingAt" => ok |input: NestingInput| nesting_at(input);
        "namedSubagent" => ok |input: NamedSubagentInput| named_subagent(&input);
        "agentTeamsOn" => ok |value: Option<String>| agent_teams_on(value.as_deref());
    }
}
