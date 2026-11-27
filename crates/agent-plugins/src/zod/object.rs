//! `z.object` (its three kinds: strip, loose, strict) and `z.record`, as zod 4 parses them: the
//! members of the shape in its order (zod's compiled fast path, which Bun runs), then the keys
//! it does not name in JavaScript's order of an object's keys, `__proto__` skipped.
//!
//! A member is read as an own member: where zod reads `input[key]` and `key in input` through
//! the prototype, a shape key such as `constructor` would find `Object.prototype`'s; the shapes
//! of the package name no such key (docs/RUST.md, "Where the crate answers differently").

use super::issue::{RawIssue, finalize};
use super::locale::to_string;
use super::params::Shape;
use super::run::{Payload, invalid_type, run};
use super::schema::{Kind, Schema};
use crate::js::{JsObject, JsValue, in_property_order, number, property_order};

/// What a member's schema answered, placed as zod's fast path places it.
fn place(field: &Schema, key: &str, present: bool, result: Payload, out: &mut Payload) {
    let (optional_in, optional_out) = (field.optional_in(), field.optional_out());
    let failed = !result.issues.is_empty();
    let skip_issues = optional_in && optional_out && !present;
    if failed && !skip_issues {
        let at = JsValue::from(key);
        out.issues.extend(
            result
                .issues
                .into_iter()
                .map(|issue| issue.under(at.clone())),
        );
    }
    if !optional_in && !present && !failed {
        let issue = RawIssue::new([
            ("code", JsValue::from("invalid_type")),
            ("expected", JsValue::from("nonoptional")),
            ("path", JsValue::Array(vec![JsValue::from(key)])),
        ]);
        out.push(issue);
    }
    let kept = if optional_in {
        present || !result.value.is_undefined()
    } else {
        present
    };
    if kept && let JsValue::Object(members) = &mut out.value {
        members.insert(key.to_owned(), result.value);
    }
}

/// `handleCatchall`: each key the shape does not name, refused by `never` or run through the
/// catch-all schema.
fn catchall_keys(
    schema: &Schema,
    shape: &Shape,
    catchall: &Schema,
    input: &JsObject,
    out: &mut Payload,
) {
    let mut unrecognized = Vec::new();
    let refuses = matches!(catchall.kind(), Kind::Never);
    for (key, value) in property_order(input) {
        if key == "__proto__" || shape.contains_key(key) {
            continue;
        }
        if refuses {
            unrecognized.push(JsValue::from(key.as_str()));
            continue;
        }
        let result = run(catchall, Payload::of(value.clone()));
        place(catchall, key, true, result, out);
    }
    if !unrecognized.is_empty() {
        let issue = RawIssue::new([
            ("code", JsValue::from("unrecognized_keys")),
            ("keys", JsValue::Array(unrecognized)),
        ])
        .input(JsValue::Object(input.clone()))
        .raised_by(schema.0.error.clone());
        out.push(issue);
    }
}

/// `$ZodObjectJIT`'s parse.
pub(crate) fn object(
    schema: &Schema,
    shape: &Shape,
    catchall: Option<&Schema>,
    payload: Payload,
) -> Payload {
    let JsValue::Object(input) = payload.value else {
        let mut payload = payload;
        let value = payload.value.clone();
        payload.push(invalid_type(schema, "object", value));
        return payload;
    };
    let mut out = Payload {
        value: JsValue::Object(JsObject::new()),
        ..payload
    };
    for (key, field) in shape {
        let member = input.get(key);
        let result = run(field, Payload::of(member.cloned().unwrap_or_default()));
        place(field, key, member.is_some(), result, &mut out);
    }
    if let Some(catchall) = catchall {
        catchall_keys(schema, shape, catchall, &input, &mut out);
    }
    if let JsValue::Object(members) = &mut out.value {
        *members = in_property_order(std::mem::take(members));
    }
    out
}

fn invalid_key(schema: &Schema, key: &str, issues: Vec<RawIssue>) -> RawIssue {
    let finalized: Vec<JsValue> = issues
        .into_iter()
        .map(|issue| finalize(issue).to_js_value())
        .collect();
    RawIssue::new([
        ("code", JsValue::from("invalid_key")),
        ("origin", JsValue::from("record")),
        ("issues", JsValue::Array(finalized)),
        ("path", JsValue::Array(vec![JsValue::from(key)])),
    ])
    .input(JsValue::from(key))
    .raised_by(schema.0.error.clone())
}

/// `regexes.number`: `/^-?\d+(?:\.\d+)?$/`, the keys a record retries as numbers.
fn numeric(key: &str) -> bool {
    let digits = key.strip_prefix('-').unwrap_or(key);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "1"));
    let all_digits =
        |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    all_digits(whole) && all_digits(fraction)
}

/// One member of a record: its key run through the key schema (a numeric key retried as a
/// number), its value through the value schema, placed under the key's output.
fn record_member(
    schema: &Schema,
    key_schema: &Schema,
    value_schema: &Schema,
    key: &str,
    value: &JsValue,
    out: &mut Payload,
) {
    let mut key_result = run(key_schema, Payload::of(JsValue::from(key)));
    if !key_result.issues.is_empty() && numeric(key) {
        let retried = run(key_schema, Payload::of(JsValue::Number(number(key))));
        if retried.issues.is_empty() {
            key_result = retried;
        }
    }
    if !key_result.issues.is_empty() {
        out.push(invalid_key(schema, key, key_result.issues));
        return;
    }
    let result = run(value_schema, Payload::of(value.clone()));
    let at = JsValue::from(key);
    out.issues.extend(
        result
            .issues
            .into_iter()
            .map(|issue| issue.under(at.clone())),
    );
    if let JsValue::Object(members) = &mut out.value {
        members.insert(to_string(&key_result.value), result.value);
    }
}

/// A record whose keys are a few values (an enum, literals): every one of them, each run through
/// the key schema, then the keys of the input that are none of them, refused.
fn exhaustive(
    schema: &Schema,
    key_schema: &Schema,
    value_schema: &Schema,
    input: &JsObject,
    out: &mut Payload,
) {
    let mut named: Vec<String> = Vec::new();
    for key in key_schema.values().unwrap_or_default() {
        if !matches!(key, JsValue::String(_) | JsValue::Number(_)) {
            continue;
        }
        let text = to_string(&key);
        named.push(text.clone());
        let key_result = run(key_schema, Payload::of(key));
        if !key_result.issues.is_empty() {
            out.push(invalid_key(schema, &text, key_result.issues));
            continue;
        }
        let value = input.get(&text).cloned().unwrap_or_default();
        let result = run(value_schema, Payload::of(value));
        let at = JsValue::from(text.as_str());
        out.issues.extend(
            result
                .issues
                .into_iter()
                .map(|issue| issue.under(at.clone())),
        );
        if let JsValue::Object(members) = &mut out.value {
            members.insert(to_string(&key_result.value), result.value);
        }
    }
    let unrecognized: Vec<JsValue> = property_order(input)
        .into_iter()
        .filter(|(key, _)| !named.contains(key))
        .map(|(key, _)| JsValue::from(key.as_str()))
        .collect();
    if !unrecognized.is_empty() {
        let issue = RawIssue::new([
            ("code", JsValue::from("unrecognized_keys")),
            ("keys", JsValue::Array(unrecognized)),
        ])
        .input(JsValue::Object(input.clone()))
        .raised_by(schema.0.error.clone());
        out.push(issue);
    }
}

/// `$ZodRecord`'s parse.
pub(crate) fn record(schema: &Schema, key: &Schema, value: &Schema, payload: Payload) -> Payload {
    let JsValue::Object(input) = payload.value else {
        let mut payload = payload;
        let found = payload.value.clone();
        payload.push(invalid_type(schema, "record", found));
        return payload;
    };
    let mut out = Payload {
        value: JsValue::Object(JsObject::new()),
        ..payload
    };
    if key.values().is_some() {
        exhaustive(schema, key, value, &input, &mut out);
    } else {
        for (name, member) in property_order(&input) {
            if name != "__proto__" {
                record_member(schema, key, value, name, member, &mut out);
            }
        }
    }
    if let JsValue::Object(members) = &mut out.value {
        *members = in_property_order(std::mem::take(members));
    }
    out
}
