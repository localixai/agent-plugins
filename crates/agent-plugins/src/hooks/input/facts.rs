//! Port of `src/hooks/input/facts.ts`, switched.
//!
//! The schema of the facts of every event, one entry per member of the official `HookEvent`
//! union. The schemas stay in the core: a schema cannot cross, so the package reads an event's
//! input through `parseFacts`.

use std::sync::LazyLock;

use super::{facts_agents as agents, facts_misc as misc, facts_model as model};
use super::{facts_session as session, facts_tools as tools};
use crate::js::JsValue;
use crate::zod::Schema;

/// The schema of the facts of every event, in the TypeScript's order.
static FACT_SCHEMAS: [(&str, &LazyLock<Schema>); 33] = [
    ("PreToolUse", &tools::PRE_TOOL_USE_FACTS),
    ("PostToolUse", &tools::POST_TOOL_USE_FACTS),
    ("PostToolUseFailure", &tools::POST_TOOL_USE_FAILURE_FACTS),
    ("PostToolBatch", &tools::POST_TOOL_BATCH_FACTS),
    ("Notification", &misc::NOTIFICATION_FACTS),
    ("UserPromptSubmit", &session::USER_PROMPT_SUBMIT_FACTS),
    ("UserPromptExpansion", &session::USER_PROMPT_EXPANSION_FACTS),
    ("SessionStart", &session::SESSION_START_FACTS),
    ("SessionEnd", &session::SESSION_END_FACTS),
    ("Stop", &agents::STOP_FACTS),
    ("StopFailure", &agents::STOP_FAILURE_FACTS),
    ("SubagentStart", &agents::SUBAGENT_START_FACTS),
    ("SubagentStop", &agents::SUBAGENT_STOP_FACTS),
    ("PreCompact", &session::PRE_COMPACT_FACTS),
    ("PostCompact", &session::POST_COMPACT_FACTS),
    ("PreModelSwitch", &model::PRE_MODEL_SWITCH_FACTS),
    ("PostModelSwitch", &model::POST_MODEL_SWITCH_FACTS),
    ("PermissionRequest", &tools::PERMISSION_REQUEST_FACTS),
    ("PermissionDenied", &tools::PERMISSION_DENIED_FACTS),
    ("Setup", &session::SETUP_FACTS),
    ("TeammateIdle", &agents::TEAMMATE_IDLE_FACTS),
    ("TaskCreated", &agents::TASK_CREATED_FACTS),
    ("TaskCompleted", &agents::TASK_COMPLETED_FACTS),
    ("Elicitation", &misc::ELICITATION_FACTS),
    ("ElicitationResult", &misc::ELICITATION_RESULT_FACTS),
    ("ConfigChange", &misc::CONFIG_CHANGE_FACTS),
    ("WorktreeCreate", &misc::WORKTREE_CREATE_FACTS),
    ("WorktreeRemove", &misc::WORKTREE_REMOVE_FACTS),
    ("InstructionsLoaded", &misc::INSTRUCTIONS_LOADED_FACTS),
    ("CwdChanged", &misc::CWD_CHANGED_FACTS),
    ("FileChanged", &misc::FILE_CHANGED_FACTS),
    ("DirectoryAdded", &misc::DIRECTORY_ADDED_FACTS),
    ("MessageDisplay", &session::MESSAGE_DISPLAY_FACTS),
];

/// The schema of `event`'s facts; `None` for a name that is no `HookEvent`, which the TypeScript's
/// type rules out (a lookup by a name from input reads own keys only, docs/RUST.md).
#[must_use]
pub fn fact_schema_of(event: &str) -> Option<&'static Schema> {
    FACT_SCHEMAS
        .iter()
        .find(|(name, _)| *name == event)
        .map(|(_, schema)| LazyLock::force(schema))
}

/// `event`'s facts read from `input` by its schema: zod's `parse`, the facts with only the keys
/// the schema names.
///
/// # Errors
///
/// What `parse` throws: the issues as `ZodError.message` writes them; for a name that is no
/// `HookEvent`, which the TypeScript's type rules out, that it has no schema.
pub fn parse_facts(event: &str, input: &JsValue) -> Result<JsValue, String> {
    let schema = fact_schema_of(event).ok_or_else(|| format!("{event} is no hook event"))?;
    schema.parse(input).map_err(|failure| failure.to_string())
}

crate::registry::exports! {
    "hooks/input/facts" {
        "parseFacts" => fallible |event: String, input: JsValue| parse_facts(&event, &input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_has_one_schema() {
        let mut names: Vec<&str> = FACT_SCHEMAS.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), FACT_SCHEMAS.len());
        assert!(fact_schema_of("Stop").is_some());
        assert!(fact_schema_of("constructor").is_none());
    }

    #[test]
    fn parse_facts_keeps_the_named_keys_and_throws_the_issues() {
        let input = crate::js::json_parse::json_parse(
            r#"{"hook_event_name":"CwdChanged","old_cwd":"/a","new_cwd":"/b","extra":1}"#,
        );
        let facts = parse_facts("CwdChanged", &input.unwrap_or(JsValue::Undefined));
        assert!(facts.is_ok_and(|facts| facts.get("extra").is_none()));
        let failure = parse_facts("CwdChanged", &JsValue::Null)
            .err()
            .unwrap_or_default();
        assert!(failure.starts_with('['), "{failure}");
        assert!(parse_facts("constructor", &JsValue::Null).is_err());
    }
}
