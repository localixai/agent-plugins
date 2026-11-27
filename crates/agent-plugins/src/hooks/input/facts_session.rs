//! Port of `src/hooks/input/facts-session.ts` at sha256:08e5cbcd7323.
//!
//! Facts of the session and prompt events (hooks.md, "SessionStart input", "Setup input",
//! "SessionEnd", "PreCompact", "PostCompact", "UserPromptSubmit input", "UserPromptExpansion input",
//! "MessageDisplay input").
//!
//! Each schema is bound to the official input in the TypeScript (`satisfies
//! z.ZodType<EventFactsOf<E>>`); here it is the same schema, written line by line.

use std::sync::LazyLock;

use super::facts_shared::{COUNT, optional};
use crate::zod::{self, Schema, fields};

/// The facts of `SessionStart`.
pub static SESSION_START_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "source" => zod::r#enum(["startup", "resume", "clear", "compact", "fork"]),
        "model" => optional(&zod::string()),
        "session_title" => optional(&zod::string()),
        "seconds_since_last_response" => optional(&zod::number()),
        "context_tokens" => optional(&COUNT),
        "prompt_cache_likely_expired" => optional(&zod::boolean()),
        "estimated_cache_write_usd" => optional(&zod::number()),
    })
});

/// The facts of `Setup`.
pub static SETUP_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "trigger" => zod::r#enum(["init", "maintenance"]),
    })
});

/// The facts of `SessionEnd`.
pub static SESSION_END_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "reason" => zod::r#enum(["clear", "resume", "logout", "prompt_input_exit", "other"]),
    })
});

/// The facts of `PreCompact`.
pub static PRE_COMPACT_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "trigger" => zod::r#enum(["manual", "auto"]),
        "custom_instructions" => zod::string().nullable(),
    })
});

/// The facts of `PostCompact`.
pub static POST_COMPACT_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "trigger" => zod::r#enum(["manual", "auto"]),
        "compact_summary" => zod::string(),
    })
});

/// The facts of `UserPromptSubmit`.
pub static USER_PROMPT_SUBMIT_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "prompt" => zod::string(),
        "source" => optional(
            &zod::r#enum(["user", "sdk", "system", "loop_wakeup", "schedule_wakeup", "poll_event"]),
        ),
        "session_title" => optional(&zod::string()),
    })
});

/// The facts of `UserPromptExpansion`.
pub static USER_PROMPT_EXPANSION_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "expansion_type" => zod::r#enum(["slash_command", "mcp_prompt"]),
        "command_name" => zod::string(),
        "command_args" => zod::string(),
        "command_source" => optional(&zod::string()),
        "prompt" => zod::string(),
    })
});

/// The facts of `MessageDisplay`.
pub static MESSAGE_DISPLAY_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "turn_id" => zod::string(),
        "message_id" => zod::string(),
        // "Zero-based index of this batch within the message".
        "index" => COUNT.clone(),
        "final" => zod::boolean(),
        // May be empty: "the final batch's delta is empty when the message ends on a newline".
        "delta" => zod::string(),
    })
});
