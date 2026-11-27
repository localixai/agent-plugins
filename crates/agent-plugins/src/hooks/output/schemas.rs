//! The core's counterpart of `src/hooks/output/schemas.ts` at sha256:a9f2a64a0a37, which stays TypeScript:
//! the type `CheckedOutput` alone, so there is nothing to call across the boundary (docs/RUST.md,
//! "What stays TypeScript"). The schemas it held are here, and only the core uses them: a schema
//! cannot cross, and the one module that checked an output with them (`parse`) runs in the core.
//!
//! The JSON output of each event: the universal fields and an optional `hookSpecificOutput` that
//! "requires a `hookEventName` field set to the event name" (hooks.md, "JSON output"), so one
//! event's object does not validate on another.
//!
//! Judgment (spec/judgments.md): the official types have no `hookSpecificOutput` member for
//! `ConfigChange`, `PreCompact` and the other events the docs give none; an object for them carries
//! only `hookEventName`, which keeps an event name that matches and drops what follows.
//!
//! A `CheckedOutput` (known fields only) is the value the schema answered, a [`crate::js::JsValue`] object.

use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::hooks::output::schemas_common::common_fields_of;
use crate::hooks::output::schemas_elicitation::{
    ELICITATION_RESULT_SPECIFIC, ELICITATION_SPECIFIC,
};
use crate::hooks::output::schemas_misc::{
    CWD_CHANGED_SPECIFIC, FILE_CHANGED_SPECIFIC, POST_MODEL_SWITCH_SPECIFIC,
    PRE_MODEL_SWITCH_SPECIFIC, WORKTREE_CREATE_SPECIFIC,
};
use crate::hooks::output::schemas_session::{
    MESSAGE_DISPLAY_SPECIFIC, NOTIFICATION_SPECIFIC, SESSION_START_SPECIFIC, SETUP_SPECIFIC,
    STOP_SPECIFIC, SUBAGENT_START_SPECIFIC, SUBAGENT_STOP_SPECIFIC, USER_PROMPT_EXPANSION_SPECIFIC,
    USER_PROMPT_SUBMIT_SPECIFIC,
};
use crate::hooks::output::schemas_tool::{
    PERMISSION_DENIED_SPECIFIC, PERMISSION_REQUEST_SPECIFIC, POST_TOOL_BATCH_SPECIFIC,
    POST_TOOL_USE_FAILURE_SPECIFIC, POST_TOOL_USE_SPECIFIC, PRE_TOOL_USE_SPECIFIC,
};
use crate::protocol::hook_events::HOOK_EVENTS;
use crate::zod::{self, Schema, fields};

fn bare(event: &'static str) -> Schema {
    zod::object(fields! { "hookEventName" => zod::literal(event) })
}

/// One row per event of the official union (`SPECIFIC_SCHEMAS`); `None` for a name that is no
/// event, where the TypeScript's lookup is `undefined`.
fn specific_schema(event: &str) -> Option<Schema> {
    let schema = match event {
        "PreToolUse" => PRE_TOOL_USE_SPECIFIC.clone(),
        "PostToolUse" => POST_TOOL_USE_SPECIFIC.clone(),
        "PostToolUseFailure" => POST_TOOL_USE_FAILURE_SPECIFIC.clone(),
        "PostToolBatch" => POST_TOOL_BATCH_SPECIFIC.clone(),
        "Notification" => NOTIFICATION_SPECIFIC.clone(),
        "UserPromptSubmit" => USER_PROMPT_SUBMIT_SPECIFIC.clone(),
        "UserPromptExpansion" => USER_PROMPT_EXPANSION_SPECIFIC.clone(),
        "SessionStart" => SESSION_START_SPECIFIC.clone(),
        "SessionEnd" => bare("SessionEnd"),
        "Stop" => STOP_SPECIFIC.clone(),
        "StopFailure" => bare("StopFailure"),
        "SubagentStart" => SUBAGENT_START_SPECIFIC.clone(),
        "SubagentStop" => SUBAGENT_STOP_SPECIFIC.clone(),
        "PreCompact" => bare("PreCompact"),
        "PostCompact" => bare("PostCompact"),
        "PreModelSwitch" => PRE_MODEL_SWITCH_SPECIFIC.clone(),
        "PostModelSwitch" => POST_MODEL_SWITCH_SPECIFIC.clone(),
        "PermissionRequest" => PERMISSION_REQUEST_SPECIFIC.clone(),
        "PermissionDenied" => PERMISSION_DENIED_SPECIFIC.clone(),
        "Setup" => SETUP_SPECIFIC.clone(),
        "TeammateIdle" => bare("TeammateIdle"),
        "TaskCreated" => bare("TaskCreated"),
        "TaskCompleted" => bare("TaskCompleted"),
        "Elicitation" => ELICITATION_SPECIFIC.clone(),
        "ElicitationResult" => ELICITATION_RESULT_SPECIFIC.clone(),
        "ConfigChange" => bare("ConfigChange"),
        "WorktreeCreate" => WORKTREE_CREATE_SPECIFIC.clone(),
        "WorktreeRemove" => bare("WorktreeRemove"),
        "InstructionsLoaded" => bare("InstructionsLoaded"),
        "CwdChanged" => CWD_CHANGED_SPECIFIC.clone(),
        "FileChanged" => FILE_CHANGED_SPECIFIC.clone(),
        "DirectoryAdded" => bare("DirectoryAdded"),
        "MessageDisplay" => MESSAGE_DISPLAY_SPECIFIC.clone(),
        _ => return None,
    };
    Some(schema)
}

/// What the TypeScript threw for a name that is no event: `SPECIFIC_SCHEMAS[event]` is
/// `undefined`, and Bun's `TypeError` says so.
const NO_SPECIFIC_SCHEMA: &str =
    "undefined is not an object (evaluating 'SPECIFIC_SCHEMAS[event].exactOptional')";

fn output_schema_of(event: &str) -> Result<Schema, String> {
    let specific = specific_schema(event).ok_or_else(|| NO_SPECIFIC_SCHEMA.to_owned())?;
    Ok(zod::object(fields! {
        ..common_fields_of(event),
        "hookSpecificOutput" => specific.exact_optional(),
    }))
}

static SCHEMAS: LazyLock<IndexMap<&'static str, Schema>> = LazyLock::new(|| {
    HOOK_EVENTS
        .iter()
        .filter_map(|event| Some((*event, output_schema_of(event).ok()?)))
        .collect()
});

/// The schema of `event`'s JSON output; `Err` with what the TypeScript threw for a name that is
/// no event.
///
/// # Errors
///
/// For a name that is no hook event.
pub fn output_schema(event: &str) -> Result<Schema, String> {
    SCHEMAS
        .get(event)
        .cloned()
        .map_or_else(|| output_schema_of(event), Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_has_a_schema_and_another_name_throws() {
        assert_eq!(SCHEMAS.len(), HOOK_EVENTS.len());
        assert_eq!(
            output_schema("Nope").err().as_deref(),
            Some(NO_SPECIFIC_SCHEMA)
        );
    }
}
