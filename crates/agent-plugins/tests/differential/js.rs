//! Differential checks of `src/js.rs` against the built-ins themselves, as
//! `scripts/rust/js-oracle.ts` recorded them in the runtime Claude Code runs on.

use agent_plugins::js::url::{INVALID_URL_MESSAGE, parse};
use agent_plugins::js::{JsValue, json_parse::json_parse, number, number_to_string, trim};
use serde_json::{Value, json};

use crate::support::{check, check_fallible};

/// A number as the oracle records one: a JSON number, or `{"$number": …}` for the four JSON
/// has no text for.
fn recorded(value: f64) -> Value {
    if value.is_nan() {
        return json!({ "$number": "NaN" });
    }
    if value == 0.0 && value.is_sign_negative() {
        return json!({ "$number": "-0" });
    }
    if value.is_infinite() {
        let name = if value > 0.0 { "Infinity" } else { "-Infinity" };
        return json!({ "$number": name });
    }
    json!(value)
}

/// The oracle's number argument read back: a JSON number, or one of the four tagged.
fn argument(value: &Value) -> f64 {
    match value.get("$number").and_then(Value::as_str) {
        Some("NaN") => f64::NAN,
        Some("-0") => -0.0,
        Some("Infinity") => f64::INFINITY,
        Some("-Infinity") => f64::NEG_INFINITY,
        _ => value.as_f64().unwrap_or(f64::NAN),
    }
}

/// A value as the oracle records `JSON.parse`'s, through `JSON.stringify`: `-0` is `0` and a
/// number with no JSON text (an infinity, past a double's range) `null`.
fn stringified(value: JsValue) -> Value {
    match value {
        JsValue::Undefined | JsValue::Null => Value::Null,
        JsValue::Bool(flag) => Value::Bool(flag),
        JsValue::Number(number) if number.is_finite() => json!(number + 0.0),
        JsValue::Number(_) => Value::Null,
        JsValue::String(text) => Value::String(text),
        JsValue::Array(items) => Value::Array(items.into_iter().map(stringified).collect()),
        JsValue::Object(members) => Value::Object(
            members
                .into_iter()
                .map(|(key, item)| (key, stringified(item)))
                .collect(),
        ),
    }
}

#[test]
fn json_parse_answers_as_javascript_core() {
    check_fallible("js/json-parse", "JSON.parse", |(text,): (String,)| {
        json_parse(&text).map(stringified)
    });
}

#[test]
fn number_answers_as_javascript_core() {
    check("js/number", "Number", |(text,): (String,)| {
        recorded(number(&text))
    });
}

#[test]
fn number_to_string_answers_as_javascript_core() {
    check("js/number-to-string", "String", |(value,): (Value,)| {
        number_to_string(argument(&value))
    });
}

#[test]
fn trim_answers_as_javascript_core() {
    check("js/trim", "trim", |(text,): (String,)| {
        trim(&text).to_owned()
    });
}

#[test]
fn url_answers_as_javascript_core() {
    check_fallible("js/url", "URL", |(text,): (String,)| {
        parse(&text).ok_or(INVALID_URL_MESSAGE)
    });
}
