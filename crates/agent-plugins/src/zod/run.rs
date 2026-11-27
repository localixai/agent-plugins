//! A schema run on a value: zod's `_zod.run` (parse, then the checks) and each kind's `parse`,
//! in zod's order of work, which decides the order of the issues and which checks still run.

use super::checks::run_checks;
use super::context::{CatchContext, Context};
use super::issue::{RawIssue, finalize};
use super::schema::{Kind, Schema, same_value_zero};
use super::{object, union};
use crate::js::JsValue;

/// zod's payload: the value as it is parsed so far, the issues raised, and the two flags a pipe
/// and a transform set (`aborted`: no more checks; `fallback`: the value is a transform's).
#[derive(Debug, Default)]
pub(crate) struct Payload {
    pub(crate) value: JsValue,
    pub(crate) issues: Vec<RawIssue>,
    pub(crate) aborted: bool,
    pub(crate) fallback: bool,
}

impl Payload {
    pub(crate) fn of(value: JsValue) -> Self {
        Self {
            value,
            ..Self::default()
        }
    }

    pub(crate) fn push(&mut self, issue: RawIssue) {
        self.issues.push(issue);
    }
}

/// `schema._zod.run(payload)`: its parse, then its checks.
pub(crate) fn run(schema: &Schema, payload: Payload) -> Payload {
    let parsed = parse(schema, payload);
    if schema.0.checks.is_empty() {
        return parsed;
    }
    run_checks(parsed, &schema.0.checks)
}

/// `invalid_type` with `expected` first, as the primitive schemas raise it.
pub(crate) fn invalid_type(schema: &Schema, expected: &str, input: JsValue) -> RawIssue {
    RawIssue::new([
        ("expected", JsValue::from(expected)),
        ("code", JsValue::from("invalid_type")),
    ])
    .input(input)
    .raised_by(schema.0.error.clone())
}

fn accept_if(schema: &Schema, mut payload: Payload, accepted: bool, expected: &str) -> Payload {
    if !accepted {
        let issue = invalid_type(schema, expected, payload.value.clone());
        payload.push(issue);
    }
    payload
}

/// `z.number()`: a finite number; `NaN` and the infinities add `received`.
fn number(schema: &Schema, mut payload: Payload) -> Payload {
    let received = match payload.value {
        JsValue::Number(number) if number.is_finite() => return payload,
        JsValue::Number(number) if number.is_nan() => Some("NaN"),
        JsValue::Number(_) => Some("Infinity"),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::String(_)
        | JsValue::Array(_)
        | JsValue::Object(_) => None,
    };
    let mut issue = invalid_type(schema, "number", payload.value.clone());
    if let Some(received) = received {
        issue = issue.set("received", received);
    }
    payload.push(issue);
    payload
}

/// `z.literal` and `z.enum`: one of the values (`SameValueZero`).
fn one_of(schema: &Schema, values: &[JsValue], mut payload: Payload) -> Payload {
    if values
        .iter()
        .any(|value| same_value_zero(value, &payload.value))
    {
        return payload;
    }
    let issue = RawIssue::new([
        ("code", JsValue::from("invalid_value")),
        ("values", JsValue::Array(values.to_vec())),
    ])
    .input(payload.value.clone())
    .raised_by(schema.0.error.clone());
    payload.push(issue);
    payload
}

fn array(schema: &Schema, element: &Schema, mut payload: Payload) -> Payload {
    if !payload.value.is_array() {
        let input = payload.value.clone();
        payload.push(invalid_type(schema, "array", input));
        return payload;
    }
    let JsValue::Array(items) = std::mem::take(&mut payload.value) else {
        return payload;
    };
    let mut out = Vec::with_capacity(items.len());
    for (index, item) in items.into_iter().enumerate() {
        let result = run(element, Payload::of(item));
        #[expect(clippy::cast_precision_loss, reason = "an array index is below 2^32")]
        let key = JsValue::Number(index as f64);
        payload.issues.extend(
            result
                .issues
                .into_iter()
                .map(|issue| issue.under(key.clone())),
        );
        out.push(result.value);
    }
    payload.value = JsValue::Array(out);
    payload
}

/// `handleOptionalResult`: an optional over a schema that takes a missing value itself answers
/// `undefined` for one when that schema failed or fell back.
fn optional(inner: &Schema, payload: Payload) -> Payload {
    if inner.optional_in() {
        let missing = payload.value.is_undefined();
        let result = run(inner, payload);
        if missing && (!result.issues.is_empty() || result.fallback) {
            return Payload::default();
        }
        return result;
    }
    if payload.value.is_undefined() {
        return payload;
    }
    run(inner, payload)
}

fn non_optional(schema: &Schema, inner: &Schema, payload: Payload) -> Payload {
    let mut result = run(inner, payload);
    if result.issues.is_empty() && result.value.is_undefined() {
        let issue = RawIssue::new([
            ("code", JsValue::from("invalid_type")),
            ("expected", JsValue::from("nonoptional")),
        ])
        .raised_by(schema.0.error.clone());
        result.push(issue);
    }
    result
}

fn default(inner: &Schema, fallback: &JsValue, mut payload: Payload) -> Payload {
    if payload.value.is_undefined() {
        fallback.clone_into(&mut payload.value);
        return payload;
    }
    let mut result = run(inner, payload);
    if result.value.is_undefined() {
        fallback.clone_into(&mut result.value);
    }
    result
}

fn catch(inner: &Schema, answer: &super::schema::CatchFn, payload: Payload) -> Payload {
    let mut result = run(inner, payload);
    if !result.issues.is_empty() {
        let context = CatchContext {
            input: std::mem::take(&mut result.value),
            issues: std::mem::take(&mut result.issues)
                .into_iter()
                .map(finalize)
                .collect(),
        };
        result.value = answer(&context);
        result.fallback = true;
    }
    result
}

/// `handlePipeResult`: the second schema runs on what the first answered, unless it failed.
fn pipe(first: &Schema, last: &Schema, payload: Payload) -> Payload {
    let mut left = run(first, payload);
    if !left.issues.is_empty() {
        left.aborted = true;
        return left;
    }
    run(
        last,
        Payload {
            value: left.value,
            issues: left.issues,
            aborted: false,
            fallback: left.fallback,
        },
    )
}

fn transform(transform: &super::schema::TransformFn, mut payload: Payload) -> Payload {
    let mut context = Context::new(&payload.value, None, None);
    let out = transform(&payload.value, &mut context);
    let issues = context.into_issues();
    payload.issues.extend(issues);
    payload.value = out;
    payload.fallback = true;
    payload
}

/// `schema._zod.parse(payload)`.
fn parse(schema: &Schema, payload: Payload) -> Payload {
    match schema.kind() {
        Kind::String => {
            let accepted = payload.value.is_string();
            accept_if(schema, payload, accepted, "string")
        }
        Kind::Number => number(schema, payload),
        Kind::Boolean => {
            let accepted = payload.value.is_boolean();
            accept_if(schema, payload, accepted, "boolean")
        }
        Kind::Null => {
            let accepted = payload.value.is_null();
            accept_if(schema, payload, accepted, "null")
        }
        Kind::Never => accept_if(schema, payload, false, "never"),
        Kind::Unknown | Kind::Custom => payload,
        Kind::Literal(values) | Kind::Enum(values) => one_of(schema, values, payload),
        Kind::Array(element) => array(schema, element, payload),
        Kind::Object { shape, catchall } => {
            object::object(schema, shape, catchall.as_ref(), payload)
        }
        Kind::Record { key, value } => object::record(schema, key, value, payload),
        Kind::Union(options) => union::union(schema, options, payload),
        Kind::DiscriminatedUnion {
            discriminator,
            options,
        } => union::discriminated(schema, discriminator, options, payload),
        Kind::Optional(inner) => optional(inner, payload),
        Kind::ExactOptional(inner) => run(inner, payload),
        Kind::Nullable(_) if payload.value.is_null() => payload,
        Kind::Nullable(inner) => run(inner, payload),
        Kind::NonOptional(inner) => non_optional(schema, inner, payload),
        Kind::Default(inner, fallback) => default(inner, fallback, payload),
        Kind::Catch(inner, answer) => catch(inner, answer, payload),
        Kind::Pipe(first, last) => pipe(first, last, payload),
        Kind::Transform(function) => transform(function, payload),
        Kind::Lazy(lazy) => run(lazy.inner(), payload),
    }
}
