//! Port of `src/protocol/hook-events.ts`, switched.
//!
//! Every hook event Claude Code fires, in its documented order (hooks.md, "Hook events"). The list
//! exists because a host needs the events at runtime; its members must equal the SDK's
//! `HookEvent` exactly, which the TypeScript wrapper checks at compile time. The input and output
//! types are the SDK's and stay in the wrapper.

/// Every hook event Claude Code fires, in its documented order (hooks.md, "Hook events"). The list
/// exists because a host needs the events at runtime; its members must equal the SDK's
/// `HookEvent` exactly.
pub const HOOK_EVENTS: &[&str] = &[
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "PostToolBatch",
    "Notification",
    "UserPromptSubmit",
    "UserPromptExpansion",
    "SessionStart",
    "SessionEnd",
    "Stop",
    "StopFailure",
    "SubagentStart",
    "SubagentStop",
    "PreCompact",
    "PostCompact",
    "PreModelSwitch",
    "PostModelSwitch",
    "PermissionRequest",
    "PermissionDenied",
    "Setup",
    "TeammateIdle",
    "TaskCreated",
    "TaskCompleted",
    "Elicitation",
    "ElicitationResult",
    "ConfigChange",
    "WorktreeCreate",
    "WorktreeRemove",
    "InstructionsLoaded",
    "CwdChanged",
    "FileChanged",
    "DirectoryAdded",
    "MessageDisplay",
];

crate::registry::exports! {
    "protocol/hook-events" {
        "HOOK_EVENTS" => ok || HOOK_EVENTS;
    }
}
