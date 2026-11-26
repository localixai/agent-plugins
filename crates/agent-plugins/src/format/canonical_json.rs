//! Port of `src/format/canonical-json.ts`, switched.
//! A JSON text of a value with its object keys sorted by code unit at every depth, so two values
//! equal in content are equal text: the one identity every "same entry", "same source" and "same
//! input" comparison uses. Written as JavaScript's `JSON.stringify` writes each part.

use crate::format::listing::by_code_unit;
use crate::js::value::write_json_string;
use crate::js::{JsValue, number_to_string};

/// The canonical JSON text of `value`; `undefined` itself, which has no JSON text, is
/// `undefined`.
#[must_use]
pub fn canonical_json(value: &JsValue) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

fn write_value(value: &JsValue, out: &mut String) {
    match value {
        JsValue::Undefined => out.push_str("undefined"),
        JsValue::Null => out.push_str("null"),
        JsValue::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        JsValue::Number(number) => out.push_str(&number_text(*number)),
        JsValue::String(text) => write_json_string(text, out),
        JsValue::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                // `undefined` has no text: JSON.stringify writes such an item as `null`.
                if item.is_undefined() {
                    out.push_str("null");
                } else {
                    write_value(item, out);
                }
            }
            out.push(']');
        }
        JsValue::Object(members) => {
            let mut keys: Vec<&String> = members
                .iter()
                .filter(|(_, member)| !member.is_undefined())
                .map(|(key, _)| key)
                .collect();
            keys.sort_by(|first, second| by_code_unit(first, second));
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_json_string(key, out);
                out.push(':');
                if let Some(member) = members.get(key) {
                    write_value(member, out);
                }
            }
            out.push('}');
        }
    }
}

/// A number as `JSON.stringify` writes it: JavaScript's text (`-0` is `0`), `null` for one with
/// none.
fn number_text(number: f64) -> String {
    if number.is_finite() {
        number_to_string(number)
    } else {
        "null".to_owned()
    }
}

crate::registry::exports! {
    "format/canonical-json" {
        "canonicalJson" => ok |value: JsValue| canonical_json(&value);
    }
}
