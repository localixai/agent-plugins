//! Port of `src/hooks/input/facts-agents.ts` at sha256:18de53538dab.
//!
//! Facts of the agent, task and stop events (hooks.md, "Stop input", "StopFailure input",
//! "SubagentStart input", "SubagentStop input", "TaskCreated input", "TaskCompleted input",
//! "TeammateIdle input").

use std::sync::LazyLock;

use super::facts_shared::optional;
use crate::zod::{self, Schema, Shape, fields};

/// One entry of `background_tasks`; the per-type fields are checked in `background-tasks.ts`.
static BACKGROUND_TASK: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "id" => zod::string(),
        "type" => zod::string(),
        "status" => zod::string(),
        "description" => zod::string(),
        "command" => optional(&zod::string()),
        "agent_type" => optional(&zod::string()),
        "server" => optional(&zod::string()),
        "tool" => optional(&zod::string()),
        "name" => optional(&zod::string()),
    })
});

/// One entry of `session_crons`.
static SESSION_CRON: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "id" => zod::string(),
        "schedule" => zod::string(),
        "recurring" => zod::boolean(),
        "prompt" => zod::string(),
    })
});

static STOPPED: LazyLock<Shape> = LazyLock::new(|| {
    fields! {
        "stop_hook_active" => zod::boolean(),
        "last_assistant_message" => optional(&zod::string()),
        "background_tasks" => optional(&zod::array(BACKGROUND_TASK.clone())),
        "session_crons" => optional(&zod::array(SESSION_CRON.clone())),
    }
});

/// The facts of `Stop`.
pub static STOP_FACTS: LazyLock<Schema> = LazyLock::new(|| zod::object(STOPPED.clone()));

/// The facts of `SubagentStop`.
pub static SUBAGENT_STOP_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        ..STOPPED.clone(),
        "agent_id" => zod::string(),
        "agent_transcript_path" => zod::string(),
        "agent_type" => zod::string(),
    })
});

/// The error types of a failed turn: the docs' twelve and the SDK's `verification_required`
/// (spec/judgments.md).
const STOP_FAILURE_ERRORS: [&str; 13] = [
    "authentication_failed",
    "oauth_org_not_allowed",
    "account_on_hold",
    "verification_required",
    "billing_error",
    "rate_limit",
    "overloaded",
    "invalid_request",
    "model_not_found",
    "server_error",
    "unknown",
    "max_output_tokens",
    "cloud_credential_error",
];

/// The facts of `StopFailure`.
pub static STOP_FAILURE_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "error" => zod::r#enum(STOP_FAILURE_ERRORS),
        "error_details" => optional(&zod::string()),
        "last_assistant_message" => optional(&zod::string()),
    })
});

/// The facts of `SubagentStart`.
pub static SUBAGENT_START_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "agent_id" => zod::string(),
        "agent_type" => zod::string(),
    })
});

static TASK: LazyLock<Shape> = LazyLock::new(|| {
    fields! {
        "task_id" => zod::string(),
        "task_subject" => zod::string(),
        "task_description" => optional(&zod::string()),
        "teammate_name" => optional(&zod::string()),
        "team_name" => optional(&zod::string()),
    }
});

/// The facts of `TaskCreated`.
pub static TASK_CREATED_FACTS: LazyLock<Schema> = LazyLock::new(|| zod::object(TASK.clone()));
/// The facts of `TaskCompleted`.
pub static TASK_COMPLETED_FACTS: LazyLock<Schema> = LazyLock::new(|| zod::object(TASK.clone()));

/// The facts of `TeammateIdle`.
pub static TEAMMATE_IDLE_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "teammate_name" => zod::string(),
        "team_name" => zod::string(),
    })
});
