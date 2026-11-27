//! zod 4's English messages (`zod/v4/locales/en.js`), word for word: the message an issue gets
//! when neither it nor its schema's error map gives one.

use super::issue::RawIssue;
use crate::js::{JsValue, number_to_string};

/// The type zod names a value by in "received …": `util.parsedType`, with `nan` written `NaN`.
/// A plain object is `object` (an object read from JSON has `Object.prototype`).
pub(crate) const fn parsed_type(value: &JsValue) -> &'static str {
    match value {
        JsValue::Undefined => "undefined",
        JsValue::Null => "null",
        JsValue::Bool(_) => "boolean",
        JsValue::Number(number) if number.is_nan() => "NaN",
        JsValue::Number(_) => "number",
        JsValue::String(_) => "string",
        JsValue::Array(_) => "array",
        JsValue::Object(_) => "object",
    }
}

/// `util.stringifyPrimitive`: a string in double quotes as it is, anything else as `String(v)`.
pub(crate) fn stringify_primitive(value: &JsValue) -> String {
    match value {
        JsValue::String(text) => format!("\"{text}\""),
        other @ (JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::Array(_)
        | JsValue::Object(_)) => to_string(other),
    }
}

/// JavaScript's `String(value)` of a primitive (an array or object as `[object Object]`-like text
/// never reaches a message: zod's literals and keys are primitives).
pub(crate) fn to_string(value: &JsValue) -> String {
    match value {
        JsValue::Undefined => "undefined".to_owned(),
        JsValue::Null => "null".to_owned(),
        JsValue::Bool(flag) => flag.to_string(),
        JsValue::Number(number) => number_to_string(*number),
        JsValue::String(text) => text.clone(),
        JsValue::Array(items) => items.iter().map(to_string).collect::<Vec<_>>().join(","),
        JsValue::Object(_) => "[object Object]".to_owned(),
    }
}

fn text<'a>(issue: &'a RawIssue, key: &str) -> Option<&'a str> {
    issue.get(key).and_then(JsValue::as_str)
}

fn list<'a>(issue: &'a RawIssue, key: &str) -> &'a [JsValue] {
    issue
        .get(key)
        .and_then(JsValue::as_array)
        .map_or(&[], Vec::as_slice)
}

fn joined(values: &[JsValue], separator: &str) -> String {
    values
        .iter()
        .map(stringify_primitive)
        .collect::<Vec<_>>()
        .join(separator)
}

/// The unit a size is counted in, for an origin that has one (`Sizable`).
fn unit(origin: &str) -> Option<&'static str> {
    match origin {
        "string" => Some("characters"),
        "file" => Some("bytes"),
        "array" | "set" => Some("items"),
        "map" => Some("entries"),
        _ => None,
    }
}

fn bound(issue: &RawIssue, key: &str, inclusive: &str, exclusive: &str) -> String {
    let adjective = if matches!(issue.get("inclusive"), Some(JsValue::Bool(true))) {
        inclusive
    } else {
        exclusive
    };
    let value = issue.get(key).map(to_string).unwrap_or_default();
    format!("{adjective}{value}")
}

fn too_big(issue: &RawIssue) -> String {
    let origin = issue
        .get("origin")
        .map_or_else(|| "value".to_owned(), to_string);
    let limit = bound(issue, "maximum", "<=", "<");
    match unit(&origin) {
        Some(unit) => format!("Too big: expected {origin} to have {limit} {unit}"),
        None => format!("Too big: expected {origin} to be {limit}"),
    }
}

fn too_small(issue: &RawIssue) -> String {
    let origin = issue
        .get("origin")
        .map_or_else(|| "undefined".to_owned(), to_string);
    let limit = bound(issue, "minimum", ">=", ">");
    match unit(&origin) {
        Some(unit) => format!("Too small: expected {origin} to have {limit} {unit}"),
        None => format!("Too small: expected {origin} to be {limit}"),
    }
}

/// `FormatDictionary`, for the formats this crate raises.
fn format_name(format: &str) -> &str {
    match format {
        "url" => "URL",
        "regex" | "template_literal" => "input",
        other => other,
    }
}

fn invalid_format(issue: &RawIssue) -> String {
    let format = text(issue, "format").unwrap_or_default();
    match format {
        "starts_with" => format!(
            "Invalid string: must start with \"{}\"",
            text(issue, "prefix").unwrap_or_default()
        ),
        "ends_with" => format!(
            "Invalid string: must end with \"{}\"",
            text(issue, "suffix").unwrap_or_default()
        ),
        "includes" => format!(
            "Invalid string: must include \"{}\"",
            text(issue, "includes").unwrap_or_default()
        ),
        "regex" => format!(
            "Invalid string: must match pattern {}",
            text(issue, "pattern").unwrap_or_default()
        ),
        other => format!("Invalid {}", format_name(other)),
    }
}

fn invalid_union(issue: &RawIssue) -> String {
    let options = list(issue, "options");
    if options.is_empty() {
        return "Invalid input".to_owned();
    }
    let quoted: Vec<String> = options
        .iter()
        .map(|option| format!("'{}'", to_string(option)))
        .collect();
    format!(
        "Invalid discriminator value. Expected {}",
        quoted.join(" | ")
    )
}

fn invalid_value(issue: &RawIssue) -> String {
    match list(issue, "values") {
        [only] => format!("Invalid input: expected {}", stringify_primitive(only)),
        values => format!("Invalid option: expected one of {}", joined(values, "|")),
    }
}

/// The English message of a raw issue (en.js's `localeError`).
pub(crate) fn message(issue: &RawIssue) -> String {
    match issue.code() {
        "invalid_type" => {
            let expected = match text(issue, "expected").unwrap_or_default() {
                "nan" => "NaN",
                other => other,
            };
            let received = parsed_type(issue.input_value());
            format!("Invalid input: expected {expected}, received {received}")
        }
        "invalid_value" => invalid_value(issue),
        "too_big" => too_big(issue),
        "too_small" => too_small(issue),
        "invalid_format" => invalid_format(issue),
        "not_multiple_of" => format!(
            "Invalid number: must be a multiple of {}",
            issue.get("divisor").map(to_string).unwrap_or_default()
        ),
        "unrecognized_keys" => {
            let keys = list(issue, "keys");
            let plural = if keys.len() > 1 { "s" } else { "" };
            format!("Unrecognized key{plural}: {}", joined(keys, ", "))
        }
        "invalid_key" => format!(
            "Invalid key in {}",
            text(issue, "origin").unwrap_or("undefined")
        ),
        "invalid_union" => invalid_union(issue),
        "invalid_element" => {
            format!(
                "Invalid value in {}",
                text(issue, "origin").unwrap_or("undefined")
            )
        }
        _ => "Invalid input".to_owned(),
    }
}
