//! Port of `src/hooks/input/facts-tools.ts` at sha256:00ac01d53f42.
//!
//! Facts of the tool events (hooks.md, "PreToolUse input", "PermissionRequest input", "PostToolUse
//! input", "PostToolUseFailure input", "PostToolBatch input", "PermissionDenied input"): each schema
//! is bound to the official input with `satisfies z.ZodType<EventFactsOf<E>>`, so the SDK stays the
//! source of truth for what a field is called and typed.

use std::sync::LazyLock;

use super::facts_shared::{ANY_VALUE, MCP_SERVER, optional};
use crate::zod::{self, Schema, fields};

/// The `z.looseObject({ type: z.string() })` the TypeScript builds inside the test of
/// `permissionUpdate`, built once here: a schema is never changed, so it answers the same.
static UPDATE_TYPE: LazyLock<Schema> =
    LazyLock::new(|| zod::loose_object(fields! { "type" => zod::string() }));

/// `permission_suggestions` entries are Claude Code's own `PermissionUpdate` values, handed back
/// to it unchanged; only the discriminating `type` is checked here (the entries are validated in
/// full where a hook returns them: `parsePermissionUpdates`).
static PERMISSION_UPDATE: LazyLock<Schema> = LazyLock::new(|| {
    zod::custom(
        |value| UPDATE_TYPE.safe_parse(value).is_ok(),
        "Expected a permission update entry",
    )
});

static BATCH_CALL: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "tool_name" => zod::string(),
        "tool_input" => ANY_VALUE.clone(),
        "tool_use_id" => zod::string(),
        "tool_response" => optional(&zod::unknown()),
    })
});

/// The facts of `PreToolUse`.
pub static PRE_TOOL_USE_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "tool_name" => zod::string(),
        "tool_input" => ANY_VALUE.clone(),
        "tool_use_id" => zod::string(),
        "mcp_server" => optional(&MCP_SERVER),
    })
});

/// The facts of `PostToolUse`.
pub static POST_TOOL_USE_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "tool_name" => zod::string(),
        "tool_input" => ANY_VALUE.clone(),
        "tool_response" => ANY_VALUE.clone(),
        "tool_use_id" => zod::string(),
        "duration_ms" => optional(&zod::number()),
        "mcp_server" => optional(&MCP_SERVER),
    })
});

/// The facts of `PostToolUseFailure`.
pub static POST_TOOL_USE_FAILURE_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "tool_name" => zod::string(),
        "tool_input" => ANY_VALUE.clone(),
        "tool_use_id" => zod::string(),
        "error" => zod::string(),
        "is_interrupt" => optional(&zod::boolean()),
        "duration_ms" => optional(&zod::number()),
        "mcp_server" => optional(&MCP_SERVER),
    })
});

/// The facts of `PostToolBatch`.
pub static POST_TOOL_BATCH_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "tool_calls" => zod::array(BATCH_CALL.clone()),
    })
});

/// The facts of `PermissionRequest`.
pub static PERMISSION_REQUEST_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "tool_name" => zod::string(),
        "tool_input" => ANY_VALUE.clone(),
        "permission_suggestions" => optional(&zod::array(PERMISSION_UPDATE.clone())),
        "mcp_server" => optional(&MCP_SERVER),
    })
});

/// The facts of `PermissionDenied`.
pub static PERMISSION_DENIED_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "tool_name" => zod::string(),
        "tool_input" => ANY_VALUE.clone(),
        "tool_use_id" => zod::string(),
        "reason" => zod::string(),
        "mcp_server" => optional(&MCP_SERVER),
    })
});
