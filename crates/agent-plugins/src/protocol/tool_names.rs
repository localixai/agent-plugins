//! Port of `src/protocol/tool-names.ts`, switched.
//!
//! The tools of tools-reference.md ("Tool" column of the first table), in the table's order. The
//! docs say which tools exist and what each is called; a rule or a hook matcher uses these
//! canonical names, never the label a tool shows in the transcript (permissions.md, "Match all
//! uses of a tool").

/// The tools of tools-reference.md ("Tool" column of the first table), in the table's order. The
/// docs say which tools exist and what each is called; a rule or a hook matcher uses these
/// canonical names, never the label a tool shows in the transcript (permissions.md, "Match all
/// uses of a tool").
pub const DOCUMENTED_TOOLS: &[&str] = &[
    "Agent",
    "Artifact",
    "AskUserQuestion",
    "Bash",
    "CronCreate",
    "CronDelete",
    "CronList",
    "Edit",
    "EndConversation",
    "EnterPlanMode",
    "EnterWorktree",
    "ExitPlanMode",
    "ExitWorktree",
    "Glob",
    "Grep",
    "ListAgents",
    "ListMcpResourcesTool",
    "LSP",
    "Monitor",
    "NotebookEdit",
    "PowerShell",
    "PushNotification",
    "Read",
    "ReadMcpResourceTool",
    "RemoteTrigger",
    "ReportFindings",
    "ScheduleWakeup",
    "SendFeedback",
    "SendMessage",
    "SendUserFile",
    "ShareOnboardingGuide",
    "Skill",
    "SubagentHandback",
    "TaskCreate",
    "TaskGet",
    "TaskList",
    "TaskOutput",
    "TaskStop",
    "TaskUpdate",
    "TodoWrite",
    "ToolSearch",
    "WaitForMcpServers",
    "WebFetch",
    "WebSearch",
    "Workflow",
    "Write",
];

/// Tools the SDK's `BUILTIN_TOOL_NAMES` lists that the tools-reference table does not: the docs
/// are silent on them, so the kit names them but claims nothing about their behaviour.
pub const SDK_ONLY_TOOLS: &[&str] = &[
    "ClaudeDesign",
    "OfferChromeSetup",
    "Projects",
    "propose_skills",
    "ProposeGoal",
    "ReadMcpResourceDirTool",
    "ReadNotifications",
    "SendUserMessage",
    "ShowOnboardingRolePicker",
];

/// Tools Claude Code removed but a rule may still name: the table says `TaskOutput` was "Removed in
/// v2.1.277", and permissions.md ("Tool name wildcards") exempts removed tools from the
/// matches-no-tool warning.
pub const REMOVED_TOOLS: &[&str] = &["TaskOutput"];

crate::registry::exports! {
    "protocol/tool-names" {
        "DOCUMENTED_TOOLS" => ok || DOCUMENTED_TOOLS;
        "SDK_ONLY_TOOLS" => ok || SDK_ONLY_TOOLS;
        "REMOVED_TOOLS" => ok || REMOVED_TOOLS;
    }
}
