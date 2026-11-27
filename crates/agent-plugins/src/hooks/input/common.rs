//! Ported from `src/hooks/input/common.ts`; only the core uses it now, so it has no wrapper.
//!
//! The fields every event's input has (hooks.md, "Common input fields"). `scratchpad_dir` is in the
//! docs ("Requires Claude Code v2.1.257 or later") but not in the SDK's `BaseHookInput`, so it is
//! typed here as a documented extension (spec/judgments.md).

use serde::Serialize;

use super::permission_mode::hook_permission_mode;
use crate::format::issues::{PluginIssue, warning};
use crate::hooks::contracts::{HookEffort, HookSession};

/// The fields every event's input has, in the order the docs' example lists them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommonInput {
    /// The session's id.
    pub session_id: String,
    /// The id of the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,
    /// The transcript's path.
    pub transcript_path: String,
    /// The working directory.
    pub cwd: String,
    /// The session's scratchpad.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scratchpad_dir: Option<String>,
    /// The permission mode, for the events that have it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// The effort level, for the events with a tool-use context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<HookEffort>,
    /// The event.
    pub hook_event_name: String,
    /// The agent's id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// The agent's type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
}

/// The events whose input has `permission_mode`. The docs say "Not all events receive this field.
/// Check the JSON example in each hook event section": these are the events whose example shows it.
const PERMISSION_MODE_EVENTS: [(&str, bool); 33] = [
    ("PreToolUse", true),
    ("PostToolUse", true),
    ("PostToolUseFailure", true),
    ("PostToolBatch", true),
    ("Notification", false),
    ("UserPromptSubmit", true),
    ("UserPromptExpansion", true),
    ("SessionStart", false),
    ("SessionEnd", false),
    ("Stop", true),
    ("StopFailure", false),
    ("SubagentStart", false),
    ("SubagentStop", true),
    ("PreCompact", false),
    ("PostCompact", false),
    ("PreModelSwitch", false),
    ("PostModelSwitch", false),
    ("PermissionRequest", true),
    ("PermissionDenied", true),
    ("Setup", false),
    ("TeammateIdle", true),
    ("TaskCreated", false),
    ("TaskCompleted", true),
    ("Elicitation", false),
    ("ElicitationResult", false),
    ("ConfigChange", false),
    ("WorktreeCreate", false),
    ("WorktreeRemove", false),
    ("InstructionsLoaded", false),
    ("CwdChanged", false),
    ("FileChanged", false),
    ("DirectoryAdded", false),
    ("MessageDisplay", false),
];

/// The events with a tool-use context, which carry `effort`: "Present for `PreToolUse`,
/// `PostToolUse`, `Stop`, `SubagentStop`, and other events with a tool-use context"; absent "for
/// session-lifecycle hooks" (the SDK's note). The other events of the agentic loop that name a
/// tool call are counted in (spec/judgments.md).
const EFFORT_EVENTS: [(&str, bool); 33] = [
    ("PreToolUse", true),
    ("PostToolUse", true),
    ("PostToolUseFailure", true),
    ("PostToolBatch", true),
    ("PermissionRequest", true),
    ("PermissionDenied", true),
    ("Stop", true),
    ("SubagentStop", true),
    ("Notification", false),
    ("UserPromptSubmit", false),
    ("UserPromptExpansion", false),
    ("SessionStart", false),
    ("SessionEnd", false),
    ("StopFailure", false),
    ("SubagentStart", false),
    ("PreCompact", false),
    ("PostCompact", false),
    ("PreModelSwitch", false),
    ("PostModelSwitch", false),
    ("Setup", false),
    ("TeammateIdle", false),
    ("TaskCreated", false),
    ("TaskCompleted", false),
    ("Elicitation", false),
    ("ElicitationResult", false),
    ("ConfigChange", false),
    ("WorktreeCreate", false),
    ("WorktreeRemove", false),
    ("InstructionsLoaded", false),
    ("CwdChanged", false),
    ("FileChanged", false),
    ("DirectoryAdded", false),
    ("MessageDisplay", false),
];

/// A table's entry for `event`; an event the table does not name has none.
fn listed(table: &[(&str, bool)], event: &str) -> bool {
    table.iter().any(|(name, value)| *name == event && *value)
}

/// A value a host left empty reads as absent: an absent field is better than an empty one.
fn given(value: Option<&String>) -> Option<String> {
    value.filter(|text| !text.is_empty()).cloned()
}

/// `permission_mode` for the events that have it, and a warning for a text that names no mode.
fn permission_mode_fields(
    event: &str,
    session: &HookSession,
) -> (Option<String>, Vec<PluginIssue>) {
    let Some(raw) = session.permission_mode.as_deref() else {
        return (None, Vec::new());
    };
    if !listed(&PERMISSION_MODE_EVENTS, event) {
        return (None, Vec::new());
    }
    if let Some(mode) = hook_permission_mode(raw) {
        return (Some(mode.to_owned()), Vec::new());
    }
    let message = format!("\"{raw}\" is not a permission mode and is not sent");
    (
        None,
        vec![warning(
            &format!("hooks.{event}.input.permission_mode"),
            &message,
        )],
    )
}

fn effort_fields(event: &str, session: &HookSession) -> Option<HookEffort> {
    if !listed(&EFFORT_EVENTS, event) {
        return None;
    }
    session.effort.as_ref().map(|effort| HookEffort {
        level: effort.level.clone(),
    })
}

/// What `commonInput` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommonFields {
    /// The common fields.
    pub fields: CommonInput,
    /// A warning for a mode that is not sent.
    pub issues: Vec<PluginIssue>,
}

/// The common fields of `event`'s input from what the host knows of the session, in the order the
/// docs' example lists them. A field the host has no value for is absent from the object, as the
/// docs say of `prompt_id` ("Absent until the first user input") and `scratchpad_dir` ("Absent when
/// the session has no scratchpad").
#[must_use]
pub fn common_input(event: &str, session: &HookSession) -> CommonFields {
    let (permission_mode, issues) = permission_mode_fields(event, session);
    let agent = session.agent.as_ref();
    let fields = CommonInput {
        session_id: session.session_id.clone(),
        prompt_id: given(session.prompt_id.as_ref()),
        transcript_path: session.transcript_path.clone(),
        cwd: session.cwd.clone(),
        scratchpad_dir: given(session.scratchpad_dir.as_ref()),
        permission_mode,
        effort: effort_fields(event, session),
        hook_event_name: event.to_owned(),
        agent_id: given(agent.and_then(|agent| agent.id.as_ref())),
        agent_type: given(agent.and_then(|agent| agent.kind.as_ref())),
    };
    CommonFields { fields, issues }
}
