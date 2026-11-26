//! `JSON.stringify(value, null, space)` of a [`JsValue`]: the text JavaScript writes, which is not
//! the wire's (`JsValue::to_json_text` tags what JSON cannot spell). `undefined` has no text, an
//! object member set to it is left out and an array item set to it is `null`; a number with no
//! JSON text (`NaN`, an infinity) is `null` and `-0` is `0`. Members keep the order they are held
//! in, which for a value read by `JSON.parse` is the order JavaScript lists them in.

use super::JsValue;
use super::number_to_string;
use super::value::write_json_string;

/// The text `JSON.stringify(value)` writes, or `None` where it returns `undefined`.
#[must_use]
pub fn json_stringify(value: &JsValue) -> Option<String> {
    json_stringify_indented(value, 0)
}

/// The text `JSON.stringify(value, null, space)` writes for a `space` of that many blanks (0 to
/// 10; none for 0), or `None` where it returns `undefined`.
#[must_use]
pub fn json_stringify_indented(value: &JsValue, space: usize) -> Option<String> {
    let gap = " ".repeat(space.min(10));
    let mut out = String::new();
    write(value, &gap, "", &mut out).then_some(out)
}

/// Writes `value` at the indentation `current`; `false` (nothing written) for `undefined`.
fn write(value: &JsValue, gap: &str, current: &str, out: &mut String) -> bool {
    match value {
        JsValue::Undefined => return false,
        JsValue::Null => out.push_str("null"),
        JsValue::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        JsValue::Number(number) if number.is_finite() => out.push_str(&number_to_string(*number)),
        JsValue::Number(_) => out.push_str("null"),
        JsValue::String(text) => write_json_string(text, out),
        JsValue::Array(items) => {
            let inner = format!("{current}{gap}");
            let parts: Vec<String> = items
                .iter()
                .map(|item| {
                    let mut text = String::new();
                    if !write(item, gap, &inner, &mut text) {
                        text.push_str("null");
                    }
                    text
                })
                .collect();
            wrap(('[', ']'), &parts, gap, current, out);
        }
        JsValue::Object(members) => {
            let inner = format!("{current}{gap}");
            let separator = if gap.is_empty() { ":" } else { ": " };
            let parts: Vec<String> = members
                .iter()
                .filter_map(|(key, item)| {
                    let mut text = String::new();
                    write_json_string(key, &mut text);
                    text.push_str(separator);
                    write(item, gap, &inner, &mut text).then_some(text)
                })
                .collect();
            wrap(('{', '}'), &parts, gap, current, out);
        }
    }
    true
}

/// The parts between the brackets, on one line without a gap and one per line with it.
fn wrap(brackets: (char, char), parts: &[String], gap: &str, current: &str, out: &mut String) {
    out.push(brackets.0);
    if !parts.is_empty() {
        if gap.is_empty() {
            out.push_str(&parts.join(","));
        } else {
            let inner = format!("{current}{gap}");
            out.push('\n');
            out.push_str(&inner);
            out.push_str(&parts.join(&format!(",\n{inner}")));
            out.push('\n');
            out.push_str(current);
        }
    }
    out.push(brackets.1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    fn parsed(text: &str) -> JsValue {
        json_parse(text).unwrap_or_default()
    }

    // bun -e 'console.log(JSON.stringify({a:[1,{b:null}],c:"x\n",d:{},e:[]}))'
    // prints: {"a":[1,{"b":null}],"c":"x\n","d":{},"e":[]}
    #[test]
    fn compact_text() {
        let value = parsed(r#"{"a":[1,{"b":null}],"c":"x\n","d":{},"e":[]}"#);
        assert_eq!(
            json_stringify(&value).as_deref(),
            Some(r#"{"a":[1,{"b":null}],"c":"x\n","d":{},"e":[]}"#)
        );
    }

    // bun -e 'console.log(JSON.stringify({a:[1,{b:2}],c:{},d:[]}, null, 2))'
    // prints the object over lines, an empty one as {} or []
    #[test]
    fn indented_text() {
        let value = parsed(r#"{"a":[1,{"b":2}],"c":{},"d":[]}"#);
        let want = "{\n  \"a\": [\n    1,\n    {\n      \"b\": 2\n    }\n  ],\n  \"c\": {},\n  \"d\": []\n}";
        assert_eq!(json_stringify_indented(&value, 2).as_deref(), Some(want));
    }

    // bun -e 'console.log(JSON.stringify([undefined, NaN, -0, Infinity, 1e21]),
    //   JSON.stringify({a: undefined, b: NaN}), JSON.stringify(undefined))'
    // prints: [null,null,0,null,1e+21] {"b":null} undefined
    #[test]
    fn what_has_no_text() {
        let items = JsValue::Array(vec![
            JsValue::Undefined,
            JsValue::Number(f64::NAN),
            JsValue::Number(-0.0),
            JsValue::Number(f64::INFINITY),
            JsValue::Number(1e21),
        ]);
        assert_eq!(
            json_stringify(&items).as_deref(),
            Some("[null,null,0,null,1e+21]")
        );
        let members = JsValue::from([("a", JsValue::Undefined), ("b", JsValue::Number(f64::NAN))]);
        assert_eq!(json_stringify(&members).as_deref(), Some(r#"{"b":null}"#));
        assert_eq!(json_stringify(&JsValue::Undefined), None);
    }
}
