//! Port of `src/agents/tool-filters.ts`, switched.

use serde::{Deserialize, Serialize};

use super::permission_baseline::PermissionMode;

/// What a subagent is: a subagent, or a fork of the main conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubagentKind {
    /// A subagent of its own.
    Subagent,
    /// A fork, which "skips both filters and receives the main conversation's exact tool pool".
    Fork,
}

/// How a subagent is started, as far as its tools are concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentRun {
    /// A fork "skips both filters and receives the main conversation's exact tool pool".
    pub kind: SubagentKind,
    /// Runs in the background: the second filter applies.
    pub background: bool,
    /// At the depth limit Claude Code withholds `Agent` from every subagent.
    pub at_depth_limit: bool,
    /// The subagent's own `permissionMode`: `ExitPlanMode` stays only for `plan`.
    pub permission_mode: PermissionMode,
    /// The agent runs as the main thread (`claude --agent` or the `agent` setting).
    pub as_main_thread: bool,
    /// The host gives this subagent `SubagentHandback` (tools-reference.md: auto mode, a local
    /// subagent that is not a fork). It is granted whatever `tools` and `disallowedTools` say.
    #[serde(default)]
    pub handback: Option<bool>,
    /// The subagent is an agent team teammate (sub-agents.md, "Available tools"): in the background
    /// it also keeps the task tools and cron tools.
    #[serde(default)]
    pub teammate: Option<bool>,
    /// The subagent already runs in its own working directory (`isolation: worktree`): tools-reference.md
    /// says `ExitWorktree` is "Not available to subagents that already run in their own working directory".
    #[serde(default)]
    pub own_working_directory: Option<bool>,
}

/// The first filter, sub-agents.md "Available tools": removed "even when listed in the `tools` field".
const REMOVED_FROM_EVERY_SUBAGENT: [&str; 6] = [
    "AskUserQuestion",
    "EndConversation",
    "EnterPlanMode",
    "ScheduleWakeup",
    "WaitForMcpServers",
    "Workflow",
];

/// The built-in tools a background subagent keeps (the second filter); every MCP tool stays.
pub const BACKGROUND_BUILTIN_TOOLS: [&str; 20] = [
    "Read",
    "Grep",
    "Glob",
    "LSP",
    "Bash",
    "PowerShell",
    "Edit",
    "Write",
    "NotebookEdit",
    "WebFetch",
    "WebSearch",
    "TodoWrite",
    "Skill",
    "ToolSearch",
    "EnterWorktree",
    "ExitWorktree",
    "Monitor",
    "TaskStop",
    "SendMessage",
    "Artifact",
];

/// What makes the first filter remove a tool of this name: always, or when the run says so.
/// `None` for a tool the first filter says nothing of.
fn first_filter(tool: &str, run: SubagentRun) -> Option<bool> {
    if REMOVED_FROM_EVERY_SUBAGENT.contains(&tool) {
        return Some(true);
    }
    match tool {
        "Agent" => Some(run.at_depth_limit),
        "ExitPlanMode" => Some(run.permission_mode != PermissionMode::Plan),
        "ExitWorktree" => Some(run.own_working_directory == Some(true)),
        // Never inherited: the host grants it (`SubagentRun.handback`), which also beats `disallowedTools`.
        "SubagentHandback" => Some(true),
        _ => None,
    }
}

/// What a teammate keeps in the background on top of `BACKGROUND_BUILTIN_TOOLS`.
const TEAMMATE_BACKGROUND_TOOLS: [&str; 7] = [
    "TaskCreate",
    "TaskGet",
    "TaskList",
    "TaskUpdate",
    "CronCreate",
    "CronDelete",
    "CronList",
];

/// "Apart from `Agent` and `ExitPlanMode`, which follow the first filter's conditions wherever the subagent runs".
const FOLLOWS_FIRST_FILTER_ONLY: [&str; 2] = ["Agent", "ExitPlanMode"];

/// Whether a subagent of this run never gets the built-in tool `tool`: the first filter, then (for
/// a background run) the second. A fork and an agent running as the main thread are not narrowed
/// (spec/judgments.md).
#[must_use]
pub fn removed_from_subagent(tool: &str, run: &SubagentRun) -> bool {
    if run.as_main_thread {
        return false;
    }
    if first_filter(tool, *run) == Some(true) {
        return true;
    }
    if !run.background
        || FOLLOWS_FIRST_FILTER_ONLY.contains(&tool)
        || BACKGROUND_BUILTIN_TOOLS.contains(&tool)
    {
        return false;
    }
    !(run.teammate == Some(true) && TEAMMATE_BACKGROUND_TOOLS.contains(&tool))
}

crate::registry::exports! {
    "agents/tool-filters" {
        "BACKGROUND_BUILTIN_TOOLS" => ok || BACKGROUND_BUILTIN_TOOLS;
        "removedFromSubagent" => ok |tool: String, run: SubagentRun| removed_from_subagent(&tool, &run);
    }
}
