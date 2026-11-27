//! The checks a schema runs after it parses (`zod/v4/core/checks.js`), and zod's rule for which
//! still run (`runChecks`): after an issue that does not say `continue: true`, only the length
//! checks run, and those only on a value that has a `length` and no explicit abort.

use std::sync::Arc;

use super::context::Context;
use super::issue::RawIssue;
use super::run::Payload;
use super::schema::{Check, CheckKind};
use crate::js::{JsValue, MAX_SAFE_INTEGER, code_unit_length, number, trim, url};

/// `util.aborted(payload, from)`: the payload was aborted, or an issue from `from` on does not
/// let the checks go on.
fn aborted(payload: &Payload, from: usize) -> bool {
    payload.aborted
        || payload
            .issues
            .get(from..)
            .is_some_and(|issues| issues.iter().any(|issue| !issue.lets_checks_run()))
}

/// `util.explicitlyAborted(payload)`: aborted, or an issue says `continue: false`.
fn explicitly_aborted(payload: &Payload) -> bool {
    payload.aborted || payload.issues.iter().any(RawIssue::aborts_explicitly)
}

/// JavaScript's `value.length` for zod's length checks: a string's UTF-16 length, an array's
/// count, an object's own `length` member read as a number; `None` where it is `undefined`.
/// An object whose `length` is an array or an object reads as `NaN` here, where JavaScript
/// would turn it into a text first; no package input has one.
fn length_of(value: &JsValue) -> Option<f64> {
    #[expect(clippy::cast_precision_loss, reason = "a length is below 2^53")]
    match value {
        JsValue::String(text) => Some(code_unit_length(text) as f64),
        JsValue::Array(items) => Some(items.len() as f64),
        JsValue::Object(members) => match members.get("length")? {
            JsValue::Undefined => None,
            JsValue::Null => Some(0.0),
            JsValue::Bool(flag) => Some(f64::from(u8::from(*flag))),
            JsValue::Number(length) => Some(*length),
            JsValue::String(text) => Some(number(text)),
            JsValue::Array(_) | JsValue::Object(_) => Some(f64::NAN),
        },
        JsValue::Undefined | JsValue::Null | JsValue::Bool(_) | JsValue::Number(_) => None,
    }
}

/// `util.getLengthableOrigin`.
const fn lengthable_origin(value: &JsValue) -> &'static str {
    match value {
        JsValue::Array(_) => "array",
        JsValue::String(_) => "string",
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::Object(_) => "unknown",
    }
}

/// `runChecks`.
pub(crate) fn run_checks(mut payload: Payload, checks: &[Check]) -> Payload {
    let mut stopped = aborted(&payload, 0);
    for check in checks {
        if check.runs_on_length() {
            if explicitly_aborted(&payload) || length_of(&payload.value).is_none() {
                continue;
            }
        } else if stopped {
            continue;
        }
        let before = payload.issues.len();
        run_check(check, &mut payload);
        if payload.issues.len() != before && !stopped {
            stopped = aborted(&payload, before);
        }
    }
    payload
}

fn raise(check: &Check, payload: &mut Payload, members: RawIssue) {
    let issue = members
        .input(payload.value.clone())
        .raised_by(check.error.clone())
        .continues(!check.abort);
    payload.push(issue);
}

fn length(check: &Check, payload: &mut Payload, limit: f64, minimum: bool) {
    let length = length_of(&payload.value).unwrap_or(f64::NAN);
    let within = if minimum {
        length >= limit
    } else {
        length <= limit
    };
    if within {
        return;
    }
    let origin = JsValue::from(lengthable_origin(&payload.value));
    let (code, key) = if minimum {
        ("too_small", "minimum")
    } else {
        ("too_big", "maximum")
    };
    let members = RawIssue::new([
        ("origin", origin),
        ("code", JsValue::from(code)),
        (key, JsValue::Number(limit)),
        ("inclusive", JsValue::Bool(true)),
    ]);
    raise(check, payload, members);
}

fn compare(check: &Check, payload: &mut Payload, value: f64, inclusive: bool, greater: bool) {
    let input = payload.value.as_f64().unwrap_or(f64::NAN);
    let within = match (greater, inclusive) {
        (true, true) => input >= value,
        (true, false) => input > value,
        (false, true) => input <= value,
        (false, false) => input < value,
    };
    if within {
        return;
    }
    let (code, key) = if greater {
        ("too_small", "minimum")
    } else {
        ("too_big", "maximum")
    };
    let members = RawIssue::new([
        ("origin", JsValue::from("number")),
        ("code", JsValue::from(code)),
        (key, JsValue::Number(value)),
        ("inclusive", JsValue::Bool(inclusive)),
    ]);
    raise(check, payload, members);
}

/// zod's `safeint` number format: an integer (else an aborting `invalid_type`), within the safe
/// range (else `too_big` or `too_small` with zod's note).
fn safe_int(check: &Check, payload: &mut Payload) {
    let input = payload.value.as_f64().unwrap_or(f64::NAN);
    if !(input.is_finite() && input.fract() == 0.0) {
        let issue = RawIssue::new([
            ("expected", JsValue::from("int")),
            ("format", JsValue::from("safeint")),
            ("code", JsValue::from("invalid_type")),
        ])
        .continues(false)
        .input(payload.value.clone())
        .raised_by(check.error.clone());
        payload.push(issue);
        return;
    }
    if input.abs() <= MAX_SAFE_INTEGER {
        return;
    }
    let (code, key, limit) = if input > 0.0 {
        ("too_big", "maximum", MAX_SAFE_INTEGER)
    } else {
        ("too_small", "minimum", -MAX_SAFE_INTEGER)
    };
    let members = RawIssue::new([
        ("code", JsValue::from(code)),
        (key, JsValue::Number(limit)),
        (
            "note",
            JsValue::from("Integers must be within the safe integer range."),
        ),
        ("origin", JsValue::from("int")),
        ("inclusive", JsValue::Bool(true)),
    ]);
    raise(check, payload, members);
}

fn regex(check: &Check, payload: &mut Payload, literal: &str, pattern: Option<&regex::Regex>) {
    let text = payload.value.as_str().unwrap_or_default();
    if pattern.is_some_and(|pattern| pattern.is_match(text)) {
        return;
    }
    let members = RawIssue::new([
        ("origin", JsValue::from("string")),
        ("code", JsValue::from("invalid_format")),
        ("format", JsValue::from("regex")),
        ("pattern", JsValue::from(literal)),
    ]);
    raise(check, payload, members);
}

/// `$ZodURL`'s check: `new URL(text.trim())`, and the trimmed text as the value.
fn url(check: &Check, payload: &mut Payload) {
    let trimmed = trim(payload.value.as_str().unwrap_or_default()).to_owned();
    if url::parse(&trimmed).is_some() {
        payload.value = JsValue::String(trimmed);
        return;
    }
    let members = RawIssue::new([
        ("code", JsValue::from("invalid_format")),
        ("format", JsValue::from("url")),
    ]);
    raise(check, payload, members);
}

/// `handleRefineResult`: a `custom` issue at the refinement's path.
fn refine(check: &Check, payload: &mut Payload, passes: bool) {
    if passes {
        return;
    }
    let members = RawIssue::new([
        ("code", JsValue::from("custom")),
        ("path", JsValue::Array(check.path.clone())),
    ]);
    raise(check, payload, members);
}

fn instance(check: &Check, payload: &mut Payload, name: &str, passes: bool) {
    if passes {
        return;
    }
    let issue = RawIssue::new([
        ("code", JsValue::from("invalid_type")),
        ("expected", JsValue::from(name)),
        ("path", JsValue::Array(check.path.clone())),
    ])
    .input(payload.value.clone())
    .raised_by(check.error.clone());
    payload.push(issue);
}

fn run_check(check: &Check, payload: &mut Payload) {
    match &check.kind {
        CheckKind::MinLength(limit) => length(check, payload, *limit, true),
        CheckKind::MaxLength(limit) => length(check, payload, *limit, false),
        CheckKind::GreaterThan { value, inclusive } => {
            compare(check, payload, *value, *inclusive, true);
        }
        CheckKind::LessThan { value, inclusive } => {
            compare(check, payload, *value, *inclusive, false);
        }
        CheckKind::SafeInt => safe_int(check, payload),
        CheckKind::Regex {
            literal,
            regex: pattern,
        } => {
            regex(check, payload, literal, pattern.as_deref());
        }
        CheckKind::Url => url(check, payload),
        CheckKind::Refine(test) => {
            let passes = test(&payload.value);
            refine(check, payload, passes);
        }
        CheckKind::Instance { name, test } => {
            let passes = test(&payload.value);
            instance(check, payload, name, passes);
        }
        CheckKind::Function(function) => {
            let mut context = Context::new(&payload.value, None, None);
            function(&mut context);
            let issues = context.into_issues();
            payload.issues.extend(issues);
        }
        CheckKind::SuperRefine(function) => {
            let error = check.error.as_ref().map(Arc::clone);
            let mut context = Context::new(&payload.value, error, Some(!check.abort));
            function(&payload.value, &mut context);
            let issues = context.into_issues();
            payload.issues.extend(issues);
        }
    }
}
