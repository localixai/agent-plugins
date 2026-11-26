//! Reading the recorded vectors and holding a Rust function to them.
//!
//! A vector line is `{"fn", "args", "outcomes", "spec", "async"?}`: the JSON arguments of one
//! call, what it returned (`{"result"}`) or threw (`{"throws"}`), more than one when the same
//! call answered differently across tests. Values JSON lacks are tagged
//! (`scripts/rust/recorder/encode.ts`): `{"$undefined": true}` reads as `null` here, the
//! `Option::None` of an argument. The other tags are left as they are: `{"$number": "-0" |
//! "NaN" | "Infinity" | "-Infinity"}`, `{"$map": [[k, v]…]}`, `{"$set": [...]}`, `{"$regexp",
//! "flags"}`, `{"$bytes": base64}`; a check that takes such a value decodes it itself.
//!
//! Numbers compare by value, as JavaScript has one number type: a Rust `3.0` answers a recorded
//! `3`. Object members compare in order: the text of the answer is the text recorded.

use std::fmt::{Display, Write as _};
use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// One recorded call.
pub(crate) struct Case {
    pub(crate) args: Value,
    /// The arguments as recorded, `undefined` still tagged: what the wrapper sends.
    pub(crate) tagged_args: Value,
    pub(crate) outcomes: Vec<Value>,
    pub(crate) spec: Vec<String>,
    /// Whether an argument itself was `undefined`, which a Rust parameter that is no `Option`
    /// cannot be.
    pub(crate) undefined_argument: bool,
}

pub(crate) fn is_undefined(value: &Value) -> bool {
    value.get("$undefined") == Some(&Value::Bool(true))
}

fn vectors_file(module: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors")
        .join(format!("{module}.jsonl"))
}

/// A recorded result as the wrapper's decoding reads it: an object member that is `undefined`
/// left out (an absent member), a top-level `undefined` kept as its tag.
pub(crate) fn as_wire(value: Value) -> Value {
    match value {
        Value::Object(members) if is_undefined(&Value::Object(members.clone())) => {
            Value::Object(members)
        }
        Value::Object(members) => Value::Object(
            members
                .into_iter()
                .filter(|(_, item)| !is_undefined(item))
                .map(|(key, item)| (key, as_wire(item)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(as_wire).collect()),
        other @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => other,
    }
}

/// `undefined` read as `null`, at every depth.
fn untag(value: Value) -> Value {
    match value {
        Value::Object(members) if is_undefined(&Value::Object(members.clone())) => Value::Null,
        Value::Object(members) => Value::Object(
            members
                .into_iter()
                .map(|(key, item)| (key, untag(item)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(untag).collect()),
        other @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => other,
    }
}

/// Every recorded call of `function` in `module` (a `src/` path without `.ts`); none when the
/// module recorded nothing.
pub(crate) fn cases_if_any(module: &str, function: &str) -> Vec<Case> {
    std::fs::read_to_string(vectors_file(module))
        .map(|text| cases_in(&text, function))
        .unwrap_or_default()
}

fn cases_in(text: &str, function: &str) -> Vec<Case> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("a vector line is JSON"))
        .filter(|line| line["fn"] == function)
        .map(|line| Case {
            undefined_argument: line["args"]
                .as_array()
                .is_some_and(|args| args.iter().any(is_undefined)),
            args: untag(line["args"].clone()),
            tagged_args: line["args"].clone(),
            outcomes: line["outcomes"].as_array().cloned().unwrap_or_default(),
            spec: serde_json::from_value(line["spec"].clone()).unwrap_or_default(),
        })
        .collect()
}

/// Every number as one spelling: an integral one that a double holds exactly as an integer, any
/// other as a double.
pub(crate) fn normalize(value: Value) -> Value {
    match value {
        Value::Number(number) => match number.as_f64() {
            Some(float) if float.fract() == 0.0 && float.abs() < 9_007_199_254_740_992.0 => {
                #[expect(clippy::cast_possible_truncation, reason = "integral and below 2^53")]
                let whole = float as i64;
                Value::from(whole)
            }
            Some(float) => Value::from(float),
            None => Value::Number(number),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(normalize).collect()),
        Value::Object(members) => Value::Object(
            members
                .into_iter()
                .map(|(key, item)| (key, normalize(item)))
                .collect(),
        ),
        other @ (Value::Null | Value::Bool(_) | Value::String(_)) => other,
    }
}

/// Equal as JSON text: the same values with object members in the same order. `Value`'s own
/// `==` ignores member order (an `IndexMap` compares as a map), and the order is part of what the
/// package answers: a host writes it out, a reader lists it.
pub(crate) fn same_text(want: &Value, got: &Value) -> bool {
    let (want, got) = (want.to_string(), got.to_string());
    want == got
}

fn expected(case: &Case) -> Vec<Value> {
    case.outcomes
        .iter()
        .map(|outcome| match outcome.get("result") {
            Some(result) => json!({ "result": normalize(untag(result.clone())) }),
            None => outcome.clone(),
        })
        .collect()
}

fn run_all(module: &str, function: &str, answer: impl Fn(Value) -> Result<Value, String>) {
    let cases = cases_if_any(module, function);
    assert!(
        !cases.is_empty(),
        "{module}#{function}: no recorded call; record its tests first"
    );
    let mut report = String::new();
    let mut failed = 0usize;
    for case in &cases {
        let got = match answer(case.args.clone()) {
            Ok(result) => json!({ "result": normalize(result) }),
            Err(message) => json!({ "throws": message }),
        };
        if !expected(case).iter().any(|want| same_text(want, &got)) {
            failed += 1;
            if failed <= 10 {
                let _ = writeln!(
                    report,
                    "  args {}\n    want {}\n    got  {got}\n    spec {:?}",
                    case.args,
                    Value::Array(case.outcomes.clone()),
                    case.spec,
                );
            }
        }
    }
    assert!(
        failed == 0,
        "{module}#{function}: {failed} of {} differ\n{report}",
        cases.len()
    );
}

/// The arguments as `A`. A JavaScript call may leave trailing optional arguments out; they are
/// `None` here, so a list shorter than the tuple is padded with `null`s until it decodes.
fn decode<A: DeserializeOwned>(args: &Value) -> Result<A, String> {
    let mut list = args.as_array().cloned().unwrap_or_default();
    let mut error = String::new();
    for _ in 0..=8 {
        match serde_json::from_value(Value::Array(list.clone())) {
            Ok(decoded) => return Ok(decoded),
            Err(failure) => error = failure.to_string(),
        }
        list.push(Value::Null);
    }
    Err(format!("arguments do not decode: {error}"))
}

fn encode<R: Serialize>(result: &R) -> Result<Value, String> {
    serde_json::to_value(result).map_err(|error| format!("result does not encode: {error}"))
}

/// Holds an infallible function to every recorded call: `A` is the argument list as a tuple.
pub(crate) fn check<A: DeserializeOwned, R: Serialize>(
    module: &str,
    function: &str,
    f: impl Fn(A) -> R,
) {
    run_all(module, function, |args| encode(&f(decode(&args)?)));
}

/// Holds a function whose original can throw: `Err` must carry the thrown message.
pub(crate) fn check_fallible<A: DeserializeOwned, R: Serialize, E: Display>(
    module: &str,
    function: &str,
    f: impl Fn(A) -> Result<R, E>,
) {
    run_all(module, function, |args| match f(decode(&args)?) {
        Ok(result) => encode(&result),
        Err(error) => Err(error.to_string()),
    });
}
