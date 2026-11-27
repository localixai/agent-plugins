//! `z.union` and `z.discriminatedUnion`, as zod 4 parses them. A union answers its first option
//! that passes; when none does and exactly one failed without aborting (only `continue: true`
//! issues, a refinement's), that option's own issues are the answer; otherwise one
//! `invalid_union` holding every option's issues.

use super::issue::{RawIssue, finalize};
use super::run::{Payload, run};
use super::schema::{Schema, same_value_zero};
use crate::js::JsValue;

/// zod's `util.aborted` of an option's result.
fn aborted(result: &Payload) -> bool {
    result.aborted || result.issues.iter().any(|issue| !issue.lets_checks_run())
}

/// `$ZodUnion`'s parse, with `handleUnionResults`.
pub(crate) fn union(schema: &Schema, options: &[Schema], mut payload: Payload) -> Payload {
    if let [only] = options {
        return run(only, payload);
    }
    let mut results = Vec::with_capacity(options.len());
    for option in options {
        let result = run(option, Payload::of(payload.value.clone()));
        if result.issues.is_empty() {
            return result;
        }
        results.push(result);
    }
    let mut standing = results
        .iter()
        .enumerate()
        .filter(|(_, result)| !aborted(result));
    if let (Some((index, _)), None) = (standing.next(), standing.next()) {
        return results.swap_remove(index);
    }
    let errors: Vec<JsValue> = results
        .into_iter()
        .map(|result| {
            JsValue::Array(
                result
                    .issues
                    .into_iter()
                    .map(|issue| finalize(issue).to_js_value())
                    .collect(),
            )
        })
        .collect();
    let issue = RawIssue::new([
        ("code", JsValue::from("invalid_union")),
        ("errors", JsValue::Array(errors)),
    ])
    .input(payload.value.clone())
    .raised_by(schema.0.error.clone());
    payload.push(issue);
    payload
}

/// The values of an option's discriminator, which pick it.
fn tags(option: &Schema, discriminator: &str) -> Vec<JsValue> {
    option
        .shape()
        .and_then(|shape| shape.get(discriminator))
        .and_then(Schema::values)
        .unwrap_or_default()
}

/// `$ZodDiscriminatedUnion`'s parse: the option whose discriminator holds the input's value,
/// or `invalid_union` at the discriminator naming every value it takes. An option without a
/// discriminator of a few values, which zod refuses when it is first used, picks nothing here;
/// a value two options take picks the first.
pub(crate) fn discriminated(
    schema: &Schema,
    discriminator: &str,
    options: &[Schema],
    mut payload: Payload,
) -> Payload {
    let Some(input) = payload.value.as_object() else {
        let issue = RawIssue::new([
            ("code", JsValue::from("invalid_type")),
            ("expected", JsValue::from("object")),
        ])
        .input(payload.value.clone())
        .raised_by(schema.0.error.clone());
        payload.push(issue);
        return payload;
    };
    let tag = input.get(discriminator).cloned().unwrap_or_default();
    let mut every: Vec<JsValue> = Vec::new();
    for option in options {
        let values = tags(option, discriminator);
        if values.iter().any(|value| same_value_zero(value, &tag)) {
            return run(option, payload);
        }
        every.extend(values);
    }
    let issue = RawIssue::new([
        ("code", JsValue::from("invalid_union")),
        ("errors", JsValue::Array(Vec::new())),
        ("note", JsValue::from("No matching discriminator")),
        ("discriminator", JsValue::from(discriminator)),
        ("options", JsValue::Array(every)),
        ("path", JsValue::Array(vec![JsValue::from(discriminator)])),
    ])
    .input(payload.value.clone())
    .raised_by(schema.0.error.clone());
    payload.push(issue);
    payload
}
