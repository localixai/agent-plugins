//! Ported from `src/hooks/output/schemas-elicitation.ts`; only the core uses it now, so it has no wrapper.
//!
//! `hookSpecificOutput` of the two elicitation events (hooks.md, "Elicitation output",
//! "ElicitationResult output"): the `action` the MCP server receives and, for `accept`, `content`.

use std::sync::LazyLock;

use crate::hooks::output::schemas_common::OBJECT_VALUE;
use crate::zod::{self, Schema, fields};

static ACTION: LazyLock<Schema> =
    LazyLock::new(|| zod::r#enum(["accept", "decline", "cancel"]).exact_optional());
static CONTENT: LazyLock<Schema> = LazyLock::new(|| OBJECT_VALUE.exact_optional());

/// The official `hookSpecificOutput` of `Elicitation`.
pub static ELICITATION_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("Elicitation"),
        "action" => ACTION.clone(),
        "content" => CONTENT.clone(),
    })
});

/// The official `hookSpecificOutput` of `ElicitationResult`.
pub static ELICITATION_RESULT_SPECIFIC: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "hookEventName" => zod::literal("ElicitationResult"),
        "action" => ACTION.clone(),
        "content" => CONTENT.clone(),
    })
});
