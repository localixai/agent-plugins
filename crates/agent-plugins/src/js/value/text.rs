//! A [`JsValue`] as the wire's JSON text, each number and string as JavaScript's
//! `JSON.stringify` writes it.

use std::fmt::Write as _;

use super::{JsValue, number_tag};
use crate::js::number_to_string;

impl JsValue {
    /// The wire's JSON text of the value: `JSON.stringify`'s text where JSON spells the value
    /// (a number as JavaScript writes it, `1` and `1e+21`), the tags where it does not
    /// (`{"$undefined":true}`, `{"$number":"NaN"}`), members in the order they are held.
    #[must_use]
    pub fn to_json_text(&self) -> String {
        let mut out = String::new();
        write_value(self, &mut out);
        out
    }
}

fn write_value(value: &JsValue, out: &mut String) {
    match value {
        JsValue::Undefined => out.push_str(r#"{"$undefined":true}"#),
        JsValue::Null => out.push_str("null"),
        JsValue::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        JsValue::Number(number) => match number_tag(*number) {
            Some(name) => {
                let _ = write!(out, r#"{{"$number":"{name}"}}"#);
            }
            None => out.push_str(&number_to_string(*number)),
        },
        JsValue::String(text) => write_json_string(text, out),
        JsValue::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        JsValue::Object(members) => {
            out.push('{');
            for (index, (key, item)) in members.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_json_string(key, out);
                out.push(':');
                write_value(item, out);
            }
            out.push('}');
        }
    }
}

/// A string quoted as `JSON.stringify` quotes it: `"` and `\` escaped, the control characters
/// below U+0020 as `\b`, `\t`, `\n`, `\f`, `\r` or `\u00xx`, every other character as itself.
pub(crate) fn write_json_string(text: &str, out: &mut String) {
    out.reserve(text.len() + 2);
    out.push('"');
    // Every run of characters that need no escape is copied whole: a call can carry a string of
    // many megabytes, and only `"`, `\` and the controls below U+0020 (all ASCII) change.
    let mut start = 0;
    for (at, byte) in text.bytes().enumerate() {
        let escape = match byte {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            0x08 => "\\b",
            b'\t' => "\\t",
            b'\n' => "\\n",
            0x0C => "\\f",
            b'\r' => "\\r",
            control if control < 0x20 => "",
            _ => continue,
        };
        out.push_str(text.get(start..at).unwrap_or_default());
        if escape.is_empty() {
            let _ = write!(out, "\\u{byte:04x}");
        } else {
            out.push_str(escape);
        }
        start = at + 1;
    }
    out.push_str(text.get(start..).unwrap_or_default());
    out.push('"');
}
