//! Ported from `src/hooks/output/schemas-tool.ts`; only the core uses it now, so it has no wrapper.
//!
//! `hookSpecificOutput` of the tool events (hooks.md, "PreToolUse decision control", "PermissionRequest
//! decision control", "PostToolUse decision control", "PostToolUseFailure decision control",
//! "PostToolBatch decision control", "PermissionDenied decision control"), each bound to the
//! official type.

use std::sync::LazyLock;

use crate::hooks::input::facts_shared::ANY_VALUE;
use crate::hooks::output::permission_update::PERMISSION_UPDATE_SCHEMA;
use crate::hooks::output::schemas_common::{FLAG, OBJECT_VALUE, TEXT};
use crate::zod::{self, Schema, Shape, fields};

static ADDITIONAL_CONTEXT: LazyLock<Shape> =
    LazyLock::new(|| fields! { "additionalContext" => TEXT.clone() });

/// The official `hookSpecificOutput` of `PreToolUse`.
pub static PRE_TOOL_USE_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("PreToolUse"),
        "permissionDecision" => zod::r#enum(["allow", "deny", "ask", "defer"]).exact_optional(),
        "permissionDecisionReason" => TEXT.clone(),
        "updatedInput" => OBJECT_VALUE.exact_optional(),
        ..ADDITIONAL_CONTEXT.clone(),
    })
});

/// "`updatedInput` ... For `\"allow\"` only", "`message` ... For `\"deny\"` only": each behavior has its own fields.
static PERMISSION_DECISION: LazyLock<Schema> = LazyLock::new(|| {
    zod::discriminated_union(
        "behavior",
        [
            zod::object(fields! {
                "behavior" => zod::literal("allow"),
                "updatedInput" => OBJECT_VALUE.exact_optional(),
                "updatedPermissions" => zod::array(PERMISSION_UPDATE_SCHEMA.clone()).exact_optional(),
            }),
            zod::object(fields! {
                "behavior" => zod::literal("deny"),
                "message" => TEXT.clone(),
                "interrupt" => FLAG.clone(),
            }),
        ],
    )
});

/// The official `hookSpecificOutput` of `PermissionRequest`.
pub static PERMISSION_REQUEST_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("PermissionRequest"),
        "decision" => PERMISSION_DECISION.clone(),
    })
});

/// The official `hookSpecificOutput` of `PostToolUse`.
pub static POST_TOOL_USE_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("PostToolUse"),
        ..ADDITIONAL_CONTEXT.clone(),
        "classifierContext" => TEXT.clone(),
        "updatedToolOutput" => ANY_VALUE.exact_optional(),
        "updatedMCPToolOutput" => ANY_VALUE.exact_optional(),
    })
});

/// The official `hookSpecificOutput` of `PostToolUseFailure`.
pub static POST_TOOL_USE_FAILURE_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("PostToolUseFailure"),
        ..ADDITIONAL_CONTEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `PostToolBatch`.
pub static POST_TOOL_BATCH_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("PostToolBatch"),
        ..ADDITIONAL_CONTEXT.clone(),
    })
});

/// The official `hookSpecificOutput` of `PermissionDenied`.
pub static PERMISSION_DENIED_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("PermissionDenied"),
        "retry" => FLAG.clone(),
    })
});
