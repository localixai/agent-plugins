//! Ported from `src/hooks/output/schemas-common.ts`; only the core uses it now, so it has no wrapper.
//!
//! The fields every event's JSON output has (hooks.md, "JSON output"): `continue`, `stopReason`,
//! `suppressOutput`, `systemMessage` and `terminalSequence`, plus the
//! top-level `decision` and `reason`. Checked against the official `SyncHookJSONOutput`.
//!
//! Judgment (spec/judgments.md): a key the schema does not name is dropped, not a failure: the docs
//! call only a value that cannot be parsed or has the wrong type a validation failure.

use std::sync::LazyLock;

use crate::hooks::input::facts_shared::optional;
use crate::zod::{self, Schema, Shape, fields};

/// "The only value for `decision` is `block`" (hooks.md, "Decision control").
static BLOCK_ONLY: LazyLock<Schema> = LazyLock::new(|| zod::literal("block"));

/// `PreToolUse` alone: "The deprecated values `approve` and `block` map to `allow` and `deny`".
static APPROVE_OR_BLOCK: LazyLock<Schema> = LazyLock::new(|| zod::r#enum(["approve", "block"]));
const APPROVING: &[&str] = &["PreToolUse"];

/// The universal fields of `event`'s output; a `decision` of another value fails validation.
#[must_use]
pub fn common_fields_of(event: &str) -> Shape {
    fields! {
        "continue" => optional(&zod::boolean()),
        "suppressOutput" => optional(&zod::boolean()),
        "stopReason" => optional(&zod::string()),
        "decision" => optional(if APPROVING.contains(&event) { &APPROVE_OR_BLOCK } else { &BLOCK_ONLY }),
        "systemMessage" => optional(&zod::string()),
        "terminalSequence" => optional(&zod::string()),
        "reason" => optional(&zod::string()),
    }
}

/// A string field the docs call optional.
pub static TEXT: LazyLock<Schema> = LazyLock::new(|| optional(&zod::string()));
/// A boolean field the docs call optional (`flag`).
pub static FLAG: LazyLock<Schema> = LazyLock::new(|| optional(&zod::boolean()));

/// A JSON object: `updatedInput` and an elicitation's `content` replace the whole object.
pub static OBJECT_VALUE: LazyLock<Schema> =
    LazyLock::new(|| zod::record(zod::string(), zod::unknown()));
