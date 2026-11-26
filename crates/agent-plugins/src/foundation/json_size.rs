//! The core's counterpart of `src/foundation/json-size.ts` at sha256:b6b0309f985a, which stays TypeScript: one native
//! `JSON.stringify` pass over a host value of any size, which crossing the boundary would copy
//! there and back (docs/RUST.md, "What stays TypeScript").
//! The size of a value written as JSON, for the core's own values.

use crate::js::{JsValue, code_unit_length, json_stringify};

/// The characters of `value` written as JSON, or `None` when nothing is written.
#[must_use]
pub fn json_chars(value: &JsValue) -> Option<usize> {
    json_stringify(value).map(|text| code_unit_length(&text))
}

/// The UTF-8 bytes of `value` written as JSON, or `None` when nothing is written.
#[must_use]
pub fn json_bytes(value: &JsValue) -> Option<usize> {
    json_stringify(value).map(|text| text.len())
}

#[cfg(test)]
mod tests {
    use super::{json_bytes, json_chars};
    use crate::js::JsValue;

    #[test]
    fn counts_code_units_and_bytes() {
        assert_eq!(json_chars(&JsValue::from("é")), Some(3));
        assert_eq!(json_bytes(&JsValue::from("é")), Some(4));
        assert_eq!(json_chars(&JsValue::Undefined), None);
    }
}
