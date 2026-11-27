//! Port of `src/hooks/input/facts-shared.ts` at sha256:97efd4af0842.
//!
//! Building blocks the per-event facts schemas share.

use std::sync::LazyLock;

use crate::zod::{self, Schema, fields};

/// A field the docs call optional or "may be absent": absent from the facts, never `undefined`.
#[must_use]
pub fn optional(schema: &Schema) -> Schema {
    schema.exact_optional()
}

/// A field typed `unknown` in the SDK (`tool_input`, `tool_response`) that the event always has:
/// any value but `undefined`, which JSON cannot carry.
pub static ANY_VALUE: LazyLock<Schema> =
    LazyLock::new(|| zod::custom(|value| !value.is_undefined(), "Required"));

/// `mcp_server`: where an MCP tool came from (the SDK's `McpServerProvenance`).
pub static MCP_SERVER: LazyLock<Schema> =
    LazyLock::new(|| zod::object(fields! { "name" => zod::string(), "source" => zod::string() }));

/// A count the docs call "Zero-based index" or tokens (`index`, `context_tokens`): a non-negative
/// integer (spec/judgments.md for the tokens; the index is the docs' own word). Other numbers the
/// docs type only as "number" stay plain numbers.
pub static COUNT: LazyLock<Schema> = LazyLock::new(|| zod::int().nonnegative());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::JsValue;

    #[test]
    fn any_value_refuses_only_undefined_with_required() {
        assert!(ANY_VALUE.safe_parse(&JsValue::Null).is_ok());
        let refused = ANY_VALUE.safe_parse(&JsValue::Undefined).err();
        let message = refused
            .as_ref()
            .and_then(|error| error.issues.first())
            .map(|issue| issue.message().to_owned());
        assert_eq!(message.as_deref(), Some("Required"));
    }

    #[test]
    fn a_count_is_a_non_negative_integer() {
        assert!(COUNT.safe_parse(&JsValue::Number(0.0)).is_ok());
        assert!(COUNT.safe_parse(&JsValue::Number(-1.0)).is_err());
        assert!(COUNT.safe_parse(&JsValue::Number(1.5)).is_err());
    }
}
