//! Port of `src/agents/tool-resolution.ts`, switched.
//!
//! The tools a subagent gets, from the definition's `tools` and `disallowedTools`, the pool the
//! main conversation offers and how the run starts.
//!
//! `pool.names` crosses as data (`HostToolNamesData`): the resolution reads only the names both
//! ways, never `callName`, so the wrapper passes the mapping's data members alone (a record that
//! holds a function would cross as a value the host keeps).

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use super::tool_filters::{SubagentKind, SubagentRun, removed_from_subagent};
use super::tool_list::{Spawn, ToolEntry, parse_tool_entry, tool_entry_matches};
use crate::protocol::tool_names::{DOCUMENTED_TOOLS, SDK_ONLY_TOOLS};
use crate::rules::tools::host_tool_names::{
    HostToolNames, HostToolNamesData, ToolNames, tool_names_of,
};
use crate::rules::types::PermissionBehavior;

/// One MCP tool of the pool: `name` is the callable name, `mcp__<server>__<tool>`.
#[derive(Clone, Debug, Deserialize)]
pub struct PoolMcpTool {
    /// The server.
    pub server: String,
    /// The callable name.
    pub name: String,
}

/// The tools the main conversation offers: what a subagent inherits from (a host passes it).
#[derive(Clone, Debug, Deserialize)]
pub struct ToolPool {
    /// The built-in tools.
    pub builtin: Vec<String>,
    /// The MCP tools.
    pub mcp: Vec<PoolMcpTool>,
    /// How the names above correspond to Claude Code's, when the host's tools carry other names:
    /// the definition's entries match the Claude Code names, the outcome names the host's tools.
    /// Absent, the names are Claude Code's.
    #[serde(default)]
    pub names: Option<HostToolNamesData>,
}

/// An agent as far as its tools go: the two frontmatter fields of the SDK's `AgentDefinition`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolFields {
    /// `tools`.
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    /// `disallowedTools`.
    #[serde(default)]
    pub disallowed_tools: Option<Vec<String>>,
    /// The agent's name.
    pub name: String,
}

/// errors.md, "Agent would be spawned with zero tools": the entries, by what went wrong.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRefusal {
    /// The message.
    pub message: String,
    /// The entry matches no tool name, usually a typo.
    pub unrecognized: Vec<String>,
    /// A real tool that subagents cannot use (a background subagent keeps fewer).
    pub not_available_to_subagents: Vec<String>,
    /// Valid, but no tool in the session matches it (no such MCP server, `Agent` at the depth
    /// limit).
    pub matched_no_tools: Vec<String>,
}

/// `any` or `none`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpawnWord {
    /// Any type.
    Any,
    /// None (without the tool).
    None,
}

/// What the `Agent` tool may spawn: a list of types, any type, or none (without the tool).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpawnTypes {
    /// `any` or `none`.
    Word(SpawnWord),
    /// A list of types.
    Types(Vec<String>),
}

/// The tools, or the refusal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ToolsOutcome {
    /// The tools.
    Granted {
        /// Always true.
        ok: bool,
        /// The tools.
        tools: Vec<String>,
        /// The subagent types the `Agent` tool may spawn; `none` without the tool.
        #[serde(rename = "spawnTypes")]
        spawn_types: SpawnTypes,
    },
    /// The refusal.
    Refused {
        /// Always false.
        ok: bool,
        /// Why.
        refusal: ToolRefusal,
    },
}

/// The refusal's groups, in `GROUPS` order: `unrecognized`, `notAvailableToSubagents`,
/// `matchedNoTools`.
const GROUP_LABELS: [&str; 3] = [
    "unrecognized",
    "not available to subagents",
    "matched no tools in this session",
];
const UNRECOGNIZED: usize = 0;
const NOT_AVAILABLE_TO_SUBAGENTS: usize = 1;
const MATCHED_NO_TOOLS: usize = 2;

fn is_known_tool(name: &str) -> bool {
    DOCUMENTED_TOOLS.contains(&name) || SDK_ONLY_TOOLS.contains(&name) || name == "Agent"
}

fn pool_names(pool: &ToolPool) -> Vec<String> {
    let mcp = pool.mcp.iter().map(|tool| tool.name.clone());
    pool.builtin.iter().cloned().chain(mcp).collect()
}

/// The tools "available to subagents": the pool minus the filters. MCP tools are never filtered.
fn available_to(pool: &ToolPool, run: SubagentRun) -> Vec<String> {
    let builtin = pool
        .builtin
        .iter()
        .filter(|tool| !removed_from_subagent(tool, &run))
        .cloned();
    builtin
        .chain(pool.mcp.iter().map(|tool| tool.name.clone()))
        .collect()
}

/// Which group of the refusal an entry that matched nothing falls into.
fn group_of(entry: &ToolEntry, pool: &ToolPool, run: SubagentRun) -> usize {
    let name = &entry.name;
    // `Agent` and any glob or MCP name are valid shapes: "If you list `Agent`, the message reports it under the next group".
    if entry.spawn.is_some() || name.contains('*') || name.starts_with("mcp__") {
        return MATCHED_NO_TOOLS;
    }
    if !is_known_tool(name) && !pool.builtin.contains(name) {
        return UNRECOGNIZED;
    }
    if removed_from_subagent(name, &run) {
        NOT_AVAILABLE_TO_SUBAGENTS
    } else {
        MATCHED_NO_TOOLS
    }
}

fn refuse(
    agent: &AgentToolFields,
    raw: &[String],
    pool: &ToolPool,
    run: SubagentRun,
) -> ToolsOutcome {
    let mut groups: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for text in raw {
        let group = group_of(&parse_tool_entry(text), pool, run);
        if let Some(list) = groups.get_mut(group) {
            list.push(text.clone());
        }
    }
    let parts: Vec<String> = GROUP_LABELS
        .iter()
        .zip(&groups)
        .filter(|(_, list)| !list.is_empty())
        .map(|(label, list)| format!("{label} [{}]", list.join(", ")))
        .collect();
    let message = format!(
        "Agent '{}' would be spawned with zero tools — refusing. Its tools list resolved to nothing: {}. Fix the agent's tools frontmatter or pass a different subagent_type.",
        agent.name,
        parts.join("; ")
    );
    let [unrecognized, not_available_to_subagents, matched_no_tools] = groups;
    ToolsOutcome::Refused {
        ok: false,
        refusal: ToolRefusal {
            message,
            unrecognized,
            not_available_to_subagents,
            matched_no_tools,
        },
    }
}

/// Which subagent types `Agent` may spawn: a type list in a subagent definition is ignored.
fn spawn_types_of(entries: Option<&[ToolEntry]>, tools: &[String], run: SubagentRun) -> SpawnTypes {
    if !tools.iter().any(|tool| tool == "Agent") {
        return SpawnTypes::Word(SpawnWord::None);
    }
    let Some(entries) = entries.filter(|_| run.as_main_thread) else {
        return SpawnTypes::Word(SpawnWord::Any);
    };
    let lists: Vec<&Spawn> = entries
        .iter()
        .filter_map(|entry| entry.spawn.as_ref())
        .collect();
    if lists.iter().any(|list| matches!(list, Spawn::Any(_))) {
        return SpawnTypes::Word(SpawnWord::Any);
    }
    let types = lists.into_iter().flat_map(|list| match list {
        Spawn::Types(types) => types.clone(),
        Spawn::Any(_) => Vec::new(),
    });
    SpawnTypes::Types(dedupe(types))
}

const HANDBACK: &str = "SubagentHandback";

fn dedupe(names: impl IntoIterator<Item = String>) -> Vec<String> {
    names
        .into_iter()
        .collect::<IndexSet<String>>()
        .into_iter()
        .collect()
}

const SEARCH_TOOLS: [&str; 2] = ["Glob", "Grep"];

/// tools-reference.md, "Glob tool behavior": "A subagent lists `Glob` or `Grep` in its `tools`
/// field and leaves out `Bash`. The listed tools come back for that subagent", whether or not the
/// session has them. A glob that covers `Bash` counts as listing it.
fn restored_search_tools(entries: Option<&[ToolEntry]>, names: &ToolNames) -> Vec<String> {
    let Some(entries) = entries else {
        return Vec::new();
    };
    let lists = |tool: &str| {
        entries
            .iter()
            .any(|entry| tool_entry_matches(entry, tool, PermissionBehavior::Allow))
    };
    // A tool the host has no tool for is not restored (`HostToolNames.missing`).
    if lists("Bash") {
        return Vec::new();
    }
    SEARCH_TOOLS
        .into_iter()
        .filter(|tool| lists(tool) && names.has(tool))
        .map(str::to_owned)
        .collect()
}

fn entries_of(agent: &AgentToolFields) -> Option<Vec<ToolEntry>> {
    agent
        .tools
        .as_ref()
        .map(|tools| tools.iter().map(|tool| parse_tool_entry(tool)).collect())
}

/// The tools a subagent could be given: the pool with the restored search tools, filtered;
/// `matchable` adds the handback the host grants.
fn candidates_of(
    agent: &AgentToolFields,
    pool: &ToolPool,
    run: SubagentRun,
    names: &ToolNames,
) -> (Vec<String>, Vec<String>) {
    let restored = restored_search_tools(entries_of(agent).as_deref(), names);
    let builtin = dedupe(pool.builtin.iter().cloned().chain(restored));
    let with_restored = ToolPool {
        builtin,
        mcp: pool.mcp.clone(),
        names: pool.names.clone(),
    };
    let available = available_to(&with_restored, run);
    // The host grants `SubagentHandback` (`SubagentRun.handback`), so an entry naming it matches.
    let mut matchable = available.clone();
    if run.handback == Some(true) {
        matchable.push(HANDBACK.to_owned());
    }
    (available, matchable)
}

fn resolve_subagent(
    agent: &AgentToolFields,
    pool: &ToolPool,
    run: SubagentRun,
    names: &ToolNames,
) -> ToolsOutcome {
    let (available, matchable) = candidates_of(agent, pool, run, names);
    let denied: Vec<ToolEntry> = agent
        .disallowed_tools
        .iter()
        .flatten()
        .map(|tool| parse_tool_entry(tool))
        .collect();
    // A host tool that stands for several Claude Code tools goes with any of its names: the grant is
    // the host's tool, so a denied `Grep` takes the `Search` that also globs (judgment).
    let siblings = |tool: &str| {
        let others = names.host_names(tool).into_iter();
        let mut all = vec![tool.to_owned()];
        all.extend(others.flat_map(|host| names.claude_names(&host)));
        all
    };
    let is_denied = |tool: &str| {
        siblings(tool).iter().any(|name| {
            denied
                .iter()
                .any(|entry| tool_entry_matches(entry, name, PermissionBehavior::Deny))
        })
    };
    let entries = entries_of(agent);
    // What each entry names in the pool, before `disallowedTools` takes its share away: a tool in
    // both fields is removed, and an entry it emptied still matched (so it never refuses).
    let matched: Vec<Vec<String>> = entries
        .iter()
        .flatten()
        .map(|entry| {
            let matches =
                |tool: &&String| tool_entry_matches(entry, tool, PermissionBehavior::Allow);
            matchable.iter().filter(matches).cloned().collect()
        })
        .collect();
    if !matched.is_empty() && matched.iter().all(Vec::is_empty) {
        return refuse(agent, agent.tools.as_deref().unwrap_or_default(), pool, run);
    }
    let listed = if entries.is_none() {
        available
    } else {
        matched.concat()
    };
    let tools: Vec<String> = dedupe(listed)
        .into_iter()
        .filter(|tool| !is_denied(tool))
        .collect();
    let granted = if run.handback == Some(true) {
        dedupe(tools.iter().cloned().chain([HANDBACK.to_owned()]))
    } else {
        tools.clone()
    };
    let spawn_types = spawn_types_of(entries.as_deref(), &tools, run);
    ToolsOutcome::Granted {
        ok: true,
        tools: granted,
        spawn_types,
    }
}

/// A fork has the main conversation's exact pool: no definition narrows it, nothing is refused.
fn resolve_fork(_agent: &AgentToolFields, pool: &ToolPool, run: SubagentRun) -> ToolsOutcome {
    let tools = pool_names(pool);
    let spawns = tools.iter().any(|tool| tool == "Agent") && !run.at_depth_limit;
    let word = if spawns {
        SpawnWord::Any
    } else {
        SpawnWord::None
    };
    ToolsOutcome::Granted {
        ok: true,
        tools,
        spawn_types: SpawnTypes::Word(word),
    }
}

/// The pool in Claude Code's names.
fn claude_pool(pool: &ToolPool, names: &ToolNames) -> ToolPool {
    let builtin = dedupe(
        pool.builtin
            .iter()
            .flat_map(|tool| names.claude_names(tool)),
    );
    let mcp = pool.mcp.iter().flat_map(|tool| {
        let server = tool.server.clone();
        names
            .claude_names(&tool.name)
            .into_iter()
            .map(move |name| PoolMcpTool {
                server: server.clone(),
                name,
            })
    });
    ToolPool {
        builtin,
        mcp: mcp.collect(),
        names: None,
    }
}

/// The tools a subagent gets (sub-agents.md, "Available tools" and "Restrict which tools a subagent
/// can use"; tools-reference.md, "Agent tool behavior"), or the refusal Claude Code gives when the
/// `tools` list resolves to nothing (errors.md). Pure over the definition's two fields, the pool the
/// host offers and how the run starts. With `pool.names`, the tools are the host's names for them.
#[must_use]
pub fn resolve_subagent_tools(
    agent: &AgentToolFields,
    pool: &ToolPool,
    run: SubagentRun,
) -> ToolsOutcome {
    let names = tool_names_of(pool.names.clone().map(|data| HostToolNames {
        to_claude: data.to_claude,
        call_name: None,
        missing: data.missing,
    }));
    let claude = claude_pool(pool, &names);
    let outcome = match run.kind {
        SubagentKind::Subagent => resolve_subagent(agent, &claude, run, &names),
        SubagentKind::Fork => resolve_fork(agent, &claude, run),
    };
    match outcome {
        ToolsOutcome::Granted {
            ok,
            tools,
            spawn_types,
        } => ToolsOutcome::Granted {
            ok,
            tools: dedupe(tools.iter().flat_map(|tool| names.host_names(tool))),
            spawn_types,
        },
        refused @ ToolsOutcome::Refused { .. } => refused,
    }
}

crate::registry::exports! {
    "agents/tool-resolution" {
        "resolveSubagentTools" => ok |agent: AgentToolFields, pool: ToolPool, run: SubagentRun|
            resolve_subagent_tools(&agent, &pool, run);
    }
}
