//! A [`JsValue`] as `serde_json` holds JSON: the value `JSON.parse(JSON.stringify(value))` gives,
//! for the one library that takes `serde_json::Value`, `flowschema` (a value checked against a
//! JSON Schema, docs/RUST.md RS8). It is what reaches a receiver that reads the value as JSON (an
//! API a tool's schema is sent to), so it is spelled as `JSON.stringify` spells it: an object
//! member set to `undefined` is left out and an array item set to it is `null`; a number with no
//! JSON text (`NaN`, an infinity) is `null`, and `-0` is `0`. A whole number of at most 2^53 is an
//! integer, as its JSON text (`1`, not `1.0`) reads back. Members keep the order they are held in.

use super::JsValue;

/// The largest whole number written without loss: `Number.MAX_SAFE_INTEGER`.
const SAFE: f64 = 9_007_199_254_740_991.0;

/// `JSON.parse(JSON.stringify(value))` as a `serde_json::Value`, or `None` where `JSON.stringify`
/// returns `undefined` (the value is `undefined`).
#[must_use]
#[expect(
    clippy::disallowed_types,
    reason = "the one place a JsValue becomes the serde_json::Value flowschema checks (RS7, RS8)"
)]
pub fn json_value(value: &JsValue) -> Option<serde_json::Value> {
    Some(match value {
        JsValue::Undefined => return None,
        JsValue::Null => serde_json::Value::Null,
        JsValue::Bool(flag) => serde_json::Value::Bool(*flag),
        JsValue::Number(number) => number_value(*number),
        JsValue::String(text) => serde_json::Value::String(text.clone()),
        JsValue::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|item| json_value(item).unwrap_or_default())
                .collect(),
        ),
        JsValue::Object(members) => serde_json::Value::Object(
            members
                .iter()
                .filter_map(|(key, member)| Some((key.clone(), json_value(member)?)))
                .collect(),
        ),
    })
}

/// A number as `JSON.stringify` writes it and JSON reads it back.
#[expect(
    clippy::disallowed_types,
    reason = "a number of the serde_json::Value json_value builds"
)]
fn number_value(number: f64) -> serde_json::Value {
    if number.fract() == 0.0 && number.abs() <= SAFE {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a whole number of at most 2^53 fits an i64 exactly"
        )]
        return serde_json::Value::from(number as i64);
    }
    serde_json::Number::from_f64(number).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

#[cfg(test)]
mod tests {
    use super::json_value;
    use crate::js::JsValue;
    use crate::js::json_parse::json_parse;

    #[test]
    fn is_the_value_json_stringify_writes() {
        let value = JsValue::from([
            ("kept", JsValue::from(1.5)),
            ("gone", JsValue::Undefined),
            (
                "items",
                JsValue::Array(vec![
                    JsValue::Undefined,
                    JsValue::Number(f64::NAN),
                    JsValue::Number(f64::INFINITY),
                    JsValue::Number(-0.0),
                    JsValue::Number(3.0),
                    JsValue::Number(1e21),
                ]),
            ),
            ("text", "é".into()),
            ("flag", JsValue::Bool(true)),
            ("none", JsValue::Null),
        ]);
        let json = json_value(&value).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"kept":1.5,"items":[null,null,null,0,3,1e+21],"text":"é","flag":true,"none":null}"#
        );
        assert!(json_value(&JsValue::Undefined).is_none());
    }

    #[test]
    fn keeps_what_json_parse_read() {
        let text = r#"{"b":[1,{"a":null}],"a":"x","n":-2.25}"#;
        let json = json_value(&json_parse(text).unwrap()).unwrap();
        assert_eq!(json.to_string(), text);
    }
}
