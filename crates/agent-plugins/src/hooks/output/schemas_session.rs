//! Ported from `src/hooks/output/schemas-session.ts`; only the core uses it now, so it has no wrapper.
//!
//! `hookSpecificOutput` of the session, prompt, stop and display events (hooks.md, "SessionStart
//! decision control", "UserPromptSubmit decision control", "UserPromptExpansion decision control",
//! "Stop decision control", "MessageDisplay output", "Setup decision control"), each bound to the
//! official type.

use std::sync::LazyLock;

use crate::hooks::output::schemas_common::{FLAG, TEXT};
use crate::zod::{self, Schema, Shape, fields};

static ADDITIONAL_CONTEXT: LazyLock<Shape> =
    LazyLock::new(|| fields! { "additionalContext" => TEXT.clone() });

/// "Array of absolute paths".
static WATCH_PATHS: LazyLock<Schema> = LazyLock::new(|| zod::array(zod::string()).exact_optional());

/// The official `hookSpecificOutput` of `SessionStart`.
pub static SESSION_START_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("SessionStart"),
        ..ADDITIONAL_CONTEXT.clone(),
        "initialUserMessage" => TEXT.clone(),
        "sessionTitle" => TEXT.clone(),
        "watchPaths" => WATCH_PATHS.clone(),
        "reloadSkills" => FLAG.clone(),
    })
});

/// The official `hookSpecificOutput` of `Setup`.
pub static SETUP_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("Setup"),
        ..ADDITIONAL_CONTEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `UserPromptSubmit`.
pub static USER_PROMPT_SUBMIT_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("UserPromptSubmit"),
        ..ADDITIONAL_CONTEXT.clone(),
        "sessionTitle" => TEXT.clone(),
        "suppressOriginalPrompt" => FLAG.clone(),
    })
});

/// The official `hookSpecificOutput` of `UserPromptExpansion`.
pub static USER_PROMPT_EXPANSION_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("UserPromptExpansion"),
        ..ADDITIONAL_CONTEXT.clone(),
        "suppressOriginalPrompt" => FLAG.clone(),
    })
});

/// The official `hookSpecificOutput` of `Stop`.
pub static STOP_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("Stop"),
        ..ADDITIONAL_CONTEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `SubagentStop`.
pub static SUBAGENT_STOP_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("SubagentStop"),
        ..ADDITIONAL_CONTEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `SubagentStart`.
pub static SUBAGENT_START_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("SubagentStart"),
        ..ADDITIONAL_CONTEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `Notification`.
pub static NOTIFICATION_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("Notification"),
        ..ADDITIONAL_CONTEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `MessageDisplay`.
pub static MESSAGE_DISPLAY_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("MessageDisplay"),
        "displayContent" => TEXT.clone(),
    })
});
