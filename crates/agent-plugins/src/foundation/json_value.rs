//! The core's counterpart of `src/foundation/json-value.ts` at sha256:6dad726da72a, which stays
//! TypeScript: it is `JSON.stringify` over any host value, a cycle, a function or a bigint
//! included, which no value crossing the boundary can be (docs/RUST.md, "What stays TypeScript").
//! The core keeps this copy for its own results (`cli/result.rs`), where every value is one a
//! `JsValue` holds.
//!
//! The TypeScript checks the parsed text with zod's `z.json()`; what `JSON.parse` gives always
//! passes it, so the core has no such check.

use crate::js::JsValue;
use crate::js::json_parse::json_parse;
use crate::js::json_stringify;

/// `value` as JSON writes it (what a file of the host would hold), or `None` (`undefined`) when
/// JSON cannot write it: a cycle, a function, a bigint, or nothing to write. A property that is
/// `undefined` is left out, as `JSON.stringify` leaves it out.
#[must_use]
pub fn to_json_value(value: &JsValue) -> Option<JsValue> {
    let text = json_stringify(value)?;
    json_parse(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::JsObject;

    #[test]
    fn writes_what_json_writes() {
        let mut members = JsObject::new();
        members.insert("a".to_owned(), JsValue::Undefined);
        members.insert("b".to_owned(), JsValue::Number(f64::NAN));
        members.insert("c".to_owned(), JsValue::Array(vec![JsValue::Undefined]));
        let mut written = JsObject::new();
        written.insert("b".to_owned(), JsValue::Null);
        written.insert("c".to_owned(), JsValue::Array(vec![JsValue::Null]));
        assert_eq!(
            to_json_value(&JsValue::Object(members)),
            Some(JsValue::Object(written))
        );
        assert_eq!(to_json_value(&JsValue::Undefined), None);
        assert_eq!(
            to_json_value(&JsValue::Number(-0.0)),
            Some(JsValue::Number(0.0))
        );
    }
}
