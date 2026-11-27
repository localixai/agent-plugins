//! Ported from `src/hooks/output/schemas-misc.ts`; only the core uses it now, so it has no wrapper.
//!
//! `hookSpecificOutput` of the remaining events that have one (hooks.md, "CwdChanged output",
//! "FileChanged output", "PreModelSwitch decision control", "PostModelSwitch decision control",
//! "WorktreeCreate output"), each bound to the official type.

use std::sync::LazyLock;

use crate::hooks::output::schemas_common::TEXT;
use crate::zod::{self, Schema, fields};

static WATCH_PATHS: LazyLock<Schema> = LazyLock::new(|| zod::array(zod::string()).exact_optional());

/// The official `hookSpecificOutput` of `CwdChanged`.
pub static CWD_CHANGED_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("CwdChanged"),
        "watchPaths" => WATCH_PATHS.clone(),
    })
});

/// The official `hookSpecificOutput` of `FileChanged`.
pub static FILE_CHANGED_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("FileChanged"),
        "watchPaths" => WATCH_PATHS.clone(),
    })
});

/// "`PreModelSwitch` accepts `allow`, `deny`, and `ask`. It doesn't accept `defer`, `updatedInput`,
/// or `additionalContext`": a `defer` fails validation, the two fields are dropped.
pub static PRE_MODEL_SWITCH_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("PreModelSwitch"),
        "permissionDecision" => zod::r#enum(["allow", "deny", "ask"]).exact_optional(),
        "permissionDecisionReason" => TEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `PostModelSwitch`.
pub static POST_MODEL_SWITCH_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("PostModelSwitch"),
        "additionalContext" => TEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `WorktreeCreate`.
pub static WORKTREE_CREATE_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("WorktreeCreate"),
        "worktreePath" => zod::string(),
    })
});
