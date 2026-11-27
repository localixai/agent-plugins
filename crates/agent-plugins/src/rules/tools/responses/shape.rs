//! Ported from `src/rules/tools/responses/shape.ts`; only the core uses it now, so it has no wrapper.
//!
//! What the readers of a PostToolUse `tool_response` share: its root, an object schema whose
//! absence and wrong type are worded as the tool tables word them, and zod's issues as the kit's.

use crate::format::issues::{PluginIssue, error};
use crate::js::string_of;
use crate::zod::{self, RawIssue, Schema, Shape, ZodError, error_fn};

/// The root of a PostToolUse payload: issues are reported at `tool_response.<field>`.
pub const RESPONSE_ROOT: &str = "tool_response";

/// An object whose absence is `is required` and whose wrong type is `expected an object`.
#[must_use]
pub fn object_of(shape: Shape) -> Schema {
    zod::object(shape).error(error_fn(|issue: &RawIssue| {
        if issue.input_value().is_undefined() {
            "is required".to_owned()
        } else {
            "expected an object".to_owned()
        }
    }))
}

/// Zod's issues as the kit's, each at `<root>.<path>`.
#[must_use]
pub fn issues_of(root: &str, failure: &ZodError) -> Vec<PluginIssue> {
    failure
        .issues
        .iter()
        .map(|found| {
            let path: Vec<String> = std::iter::once(root.to_owned())
                .chain(found.path().iter().map(string_of))
                .collect();
            error(&path.join("."), found.message())
        })
        .collect()
}
