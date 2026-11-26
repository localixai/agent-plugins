//! Differential checks of `src/js.rs` against the built-ins themselves, as
//! `scripts/rust/js-oracle.ts` recorded them in the runtime Claude Code runs on.

use agent_plugins::js::normalize::{NormalForm, normalize};
use agent_plugins::js::reg_exp::{Unrun, unicode_reg_exp};
use agent_plugins::js::url::{INVALID_URL_MESSAGE, parse, parse_with_base};
use agent_plugins::js::{JsValue, json_parse::json_parse, number, number_to_string, trim};
use serde_json::{Value, json};

use crate::support::{cases_if_any, check, check_fallible};

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

#[test]
fn url_with_a_base_answers_as_javascript_core() {
    check_fallible("js/url-base", "URL", |(text, base): (String, String)| {
        parse_with_base(&text, &base).ok_or(INVALID_URL_MESSAGE)
    });
}

#[test]
fn normalize_answers_as_javascript_core() {
    check(
        "js/normalize",
        "normalize",
        |(text, form): (String, NormalForm)| normalize(&text, form),
    );
}

#[test]
fn case_mapping_answers_as_javascript_core() {
    check("js/case", "toUpperCase", |(text,): (String,)| {
        text.to_uppercase()
    });
    check("js/case", "toLowerCase", |(text,): (String,)| {
        text.to_lowercase()
    });
}

/// Every source JavaScript refuses is refused, every one it runs answers each text as it did,
/// and the ones the `regex` crate cannot run (`Unrun::Unsupported`) are counted, never matched.
#[test]
fn unicode_reg_exp_answers_as_javascript_core() {
    let cases = cases_if_any("js/reg-exp", "RegExp");
    assert!(!cases.is_empty(), "no recorded patterns");
    let (mut wrong, mut unsupported) = (Vec::new(), 0);
    for case in &cases {
        let source = case.args[0].as_str().unwrap_or_default();
        let texts: Vec<&str> = case.args[1]
            .as_array()
            .map(|texts| texts.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let recorded = case.outcomes[0].get("result");
        let ours = unicode_reg_exp(source);
        match (ours, recorded) {
            (Err(Unrun::Invalid), None) | (Err(Unrun::Unsupported), None) => {}
            (Err(Unrun::Unsupported), Some(_)) => unsupported += 1,
            (Ok(regex), Some(want)) => {
                let got: Vec<bool> = texts.iter().map(|text| regex.is_match(text)).collect();
                if &json!(got) != want {
                    wrong.push(format!("{source:?} answers {got:?}, JavaScript {want}"));
                }
            }
            (ours, want) => wrong.push(format!("{source:?}: {ours:?}, JavaScript {want:?}")),
        }
    }
    eprintln!(
        "js/reg-exp: {unsupported} of {} not run by the regex crate",
        cases.len()
    );
    assert!(
        wrong.is_empty(),
        "{} differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
