//! Port of `src/format/frontmatter-values.ts`, switched.
//!
//! Coercions of frontmatter values, shared by skills, commands, output styles and agents. YAML 1.2
//! (the core schema `parse_frontmatter` uses) reads `yes`, `no` and `on` as plain strings, so the
//! flag spellings Claude Code accepts are handled here, not by the parser.
//!
//! Each reader takes the field's key (for the problem it may report) and the raw value, a
//! [`JsValue`] as the YAML reader gave it (`.nan`, `.inf` and `-0` are numbers there). An absent
//! value (`undefined`, or YAML `null`) is `None` with no problem; a value of the wrong shape is
//! `None` with a [`FieldProblem`], so the caller decides whether to warn or stay silent.

use serde::{Deserialize, Serialize};

use crate::js::{JsObject, JsValue, is_safe_integer, number_to_string, property_order, trim};

/// What was wrong with a field's value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldProblem {
    /// The field.
    pub key: String,
    /// Reads after the key: `${key} ${reason}`.
    pub reason: String,
}

/// What a reader found: the value, and the problem when it was dropped or kept in part.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldRead<T> {
    /// The value, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    /// Why the value is missing or not whole.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<FieldProblem>,
}

/// The reason of a value that is not a table.
pub const NOT_A_TABLE: &str = "is a list or a single value, not a table";

fn absent<T>() -> FieldRead<T> {
    FieldRead {
        value: None,
        problem: None,
    }
}

fn ok<T>(value: T) -> FieldRead<T> {
    FieldRead {
        value: Some(value),
        problem: None,
    }
}

fn bad<T>(key: &str, reason: &str) -> FieldRead<T> {
    FieldRead {
        value: None,
        problem: Some(FieldProblem {
            key: key.to_owned(),
            reason: reason.to_owned(),
        }),
    }
}

const fn is_absent(value: &JsValue) -> bool {
    value.is_nullish()
}

/// The flag spellings, in the order Claude Code lists them.
const FLAGS: [(&str, bool); 8] = [
    ("true", true),
    ("yes", true),
    ("on", true),
    ("1", true),
    ("false", false),
    ("no", false),
    ("off", false),
    ("0", false),
];
const NOT_A_FLAG: &str = "is not a flag: use true, false, yes, no, on, off, 1 or 0";

/// `String(value)` of a string or a number.
fn text_of(value: &JsValue) -> Option<String> {
    match value {
        JsValue::String(text) => Some(text.clone()),
        JsValue::Number(number) => Some(number_to_string(*number)),
        JsValue::Bool(flag) => Some(flag.to_string()),
        JsValue::Undefined | JsValue::Null | JsValue::Array(_) | JsValue::Object(_) => None,
    }
}

/// `true`/`false`/`yes`/`no`/`on`/`off`/`1`/`0`, as a YAML boolean, number or string in any case.
#[must_use]
pub fn read_flag(key: &str, value: &JsValue) -> FieldRead<bool> {
    if is_absent(value) {
        return absent();
    }
    if let JsValue::Bool(flag) = value {
        return ok(*flag);
    }
    if !matches!(value, JsValue::String(_) | JsValue::Number(_)) {
        return bad(key, NOT_A_FLAG);
    }
    let spelling = text_of(value)
        .map(|text| trim(&text).to_lowercase())
        .unwrap_or_default();
    match FLAGS.iter().find(|(name, _)| *name == spelling) {
        Some((_, flag)) => ok(*flag),
        None => bad(key, NOT_A_FLAG),
    }
}

/// A string, trimmed; a number or boolean is written out; blank is absent.
#[must_use]
pub fn read_text(key: &str, value: &JsValue) -> FieldRead<String> {
    if is_absent(value) {
        return absent();
    }
    let Some(written) = text_of(value) else {
        return bad(key, "is not text");
    };
    let text = trim(&written);
    if text.is_empty() {
        absent()
    } else {
        ok(text.to_owned())
    }
}

/// Which separators split a string list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Separators {
    /// A comma only.
    #[serde(rename = "comma")]
    Comma,
    /// A comma or any blank.
    #[serde(rename = "space-or-comma")]
    SpaceOrComma,
}

/// How a string list is split. Which one a field uses is the docs' call: `allowed-tools` and
/// `disallowed-tools` (skills.md) are `space-or-comma`; `tools` and `disallowedTools`
/// (sub-agents.md) and `paths` (skills.md, memory.md) are `comma`, so `my dir/**` stays one item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct ListOptions {
    /// What separates the items.
    pub separators: Separators,
    /// Also keep `{...}` whole, for globs such as `src/**/*.{ts,tsx}`.
    #[serde(default)]
    pub braces: Option<bool>,
}

/// Split outside parentheses (and braces, when asked): `Bash(git add *) Read` is two items.
fn split_list(text: &str, options: ListOptions) -> Vec<String> {
    let braces = options.braces.unwrap_or(false);
    let is_separator = |character: char| match options.separators {
        Separators::Comma => character == ',',
        Separators::SpaceOrComma => character == ',' || crate::js::is_whitespace(character),
    };
    let mut items: Vec<String> = Vec::new();
    let mut depth: usize = 0;
    let mut current = String::new();
    for character in text.chars() {
        if character == '(' || (braces && character == '{') {
            depth += 1;
        } else if character == ')' || (braces && character == '}') {
            depth = depth.saturating_sub(1);
        }
        if depth == 0 && is_separator(character) {
            items.push(std::mem::take(&mut current));
        } else {
            current.push(character);
        }
    }
    items.push(current);
    items
        .iter()
        .map(|item| trim(item))
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A list as Claude Code takes it: a string split as `options` say, outside parentheses, or a
/// YAML list whose items are kept whole. An item that is not text is dropped with a problem.
#[must_use]
pub fn read_list(key: &str, value: &JsValue, options: ListOptions) -> FieldRead<Vec<String>> {
    if is_absent(value) {
        return absent();
    }
    if let JsValue::String(text) = value {
        return ok(split_list(text, options));
    }
    let JsValue::Array(entries) = value else {
        return bad(
            key,
            "is not a list: use a comma-separated string or a YAML list",
        );
    };
    let items: Vec<&str> = entries.iter().filter_map(JsValue::as_str).collect();
    let kept: Vec<String> = items
        .iter()
        .map(|item| trim(item))
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect();
    if items.len() == entries.len() {
        return ok(kept);
    }
    FieldRead {
        value: Some(kept),
        problem: Some(FieldProblem {
            key: key.to_owned(),
            reason: "has an item that is not text".to_owned(),
        }),
    }
}

/// The limits of a whole number; one that is not set does not limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
pub struct IntegerBounds {
    /// The least it may be.
    #[serde(default)]
    pub min: Option<f64>,
    /// The most it may be.
    #[serde(default)]
    pub max: Option<f64>,
}

/// `^[+-]?\d+$`.
fn is_integer_text(text: &str) -> bool {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn whole_number(value: &JsValue) -> Option<f64> {
    let number = match value {
        JsValue::String(text) if is_integer_text(trim(text)) => trim(text).parse::<f64>().ok(),
        JsValue::Number(number) => Some(*number),
        JsValue::String(_)
        | JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Array(_)
        | JsValue::Object(_) => None,
    };
    number.filter(|number| is_safe_integer(*number))
}

/// `at least 1`, `at most 9` or `at least 1 and at most 9`: the bounds that are set.
fn describe_bounds(bounds: IntegerBounds) -> String {
    let mut parts = Vec::new();
    if let Some(min) = bounds.min {
        parts.push(format!("at least {}", number_to_string(min)));
    }
    if let Some(max) = bounds.max {
        parts.push(format!("at most {}", number_to_string(max)));
    }
    format!("must be {}", parts.join(" and "))
}

/// A whole number, as a YAML number or a string of digits, within `min` and `max` when given.
#[must_use]
pub fn read_integer(key: &str, value: &JsValue, bounds: IntegerBounds) -> FieldRead<f64> {
    if is_absent(value) {
        return absent();
    }
    let Some(number) = whole_number(value) else {
        return bad(key, "is not a whole number");
    };
    let min = bounds.min.unwrap_or(f64::NEG_INFINITY);
    let max = bounds.max.unwrap_or(f64::INFINITY);
    if number < min || number > max {
        bad(key, &describe_bounds(bounds))
    } else {
        ok(number)
    }
}

/// A YAML table; anything else is dropped.
#[must_use]
pub fn read_table(key: &str, value: &JsValue) -> FieldRead<JsObject> {
    if is_absent(value) {
        return absent();
    }
    match value {
        // `Object.fromEntries(Object.entries(value))`: integer-like names come first.
        JsValue::Object(table) => ok(property_order(table)
            .into_iter()
            .map(|(name, member)| (name.clone(), member.clone()))
            .collect()),
        JsValue::String(_)
        | JsValue::Number(_)
        | JsValue::Bool(_)
        | JsValue::Array(_)
        | JsValue::Undefined
        | JsValue::Null => bad(key, NOT_A_TABLE),
    }
}

/// One of `allowed`, matched exactly after trimming (no case folding).
#[must_use]
pub fn read_enum(key: &str, value: &JsValue, allowed: &[&str]) -> FieldRead<String> {
    let text = read_text(key, value);
    if let Some(problem) = text.problem {
        return bad(key, &problem.reason);
    }
    let Some(text) = text.value else {
        return absent();
    };
    match allowed.iter().find(|candidate| **candidate == text) {
        Some(found) => ok((*found).to_owned()),
        None => bad(key, &format!("is not one of: {}", allowed.join(", "))),
    }
}

crate::registry::exports! {
    "format/frontmatter-values" {
        "NOT_A_TABLE" => ok || NOT_A_TABLE;
        "readFlag" => ok |key: String, value: JsValue| read_flag(&key, &value);
        "readText" => ok |key: String, value: JsValue| read_text(&key, &value);
        "readList" => ok |key: String, value: JsValue, options: ListOptions| read_list(&key, &value, options);
        "readInteger" => ok |key: String, value: JsValue, bounds: Option<IntegerBounds>| read_integer(&key, &value, bounds.unwrap_or_default());
        "readTable" => ok |key: String, value: JsValue| read_table(&key, &value);
        "readEnum" => ok |key: String, value: JsValue, allowed: Vec<String>| {
            let allowed: Vec<&str> = allowed.iter().map(String::as_str).collect();
            read_enum(&key, &value, &allowed)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'import { readTable } from "./src/format/frontmatter-values.ts";
    //   console.log(JSON.stringify(readTable("k", JSON.parse(`{"b":1,"10":2,"2":3}`))))'
    // prints {"value":{"2":3,"10":2,"b":1}}
    #[test]
    fn a_table_lists_integer_like_names_first() {
        let table =
            crate::js::json_parse::json_parse(r#"{"b":1,"10":2,"2":3}"#).unwrap_or_default();
        let names: Vec<String> = read_table("k", &table)
            .value
            .map(|members| members.into_iter().map(|(name, _)| name).collect())
            .unwrap_or_default();
        assert_eq!(names, ["2", "10", "b"]);
    }

    // YAML's `.nan`, `.inf` and `-0` are numbers: `readInteger('maxTurns', Number.NaN)` reports
    // a problem, as Number.isSafeInteger refuses NaN and the infinities; `-0` is a whole number,
    // and `String(-0)` is `0`.
    #[test]
    fn the_numbers_json_lacks_read_as_javascript_reads_them() {
        let bounds = IntegerBounds::default();
        let not_whole = Some("is not a whole number".to_owned());
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let read = read_integer("maxTurns", &JsValue::Number(number), bounds);
            assert_eq!(
                (read.value, read.problem.map(|p| p.reason)),
                (None, not_whole.clone())
            );
        }
        let zero = read_integer("maxTurns", &JsValue::Number(-0.0), bounds).value;
        assert!(zero.is_some_and(|zero| zero == 0.0 && zero.is_sign_negative()));
        let nan = JsValue::Number(f64::NAN);
        assert_eq!(read_text("k", &nan).value.as_deref(), Some("NaN"));
        assert!(read_table("k", &nan).problem.is_some());
        assert_eq!(read_flag("k", &JsValue::Number(-0.0)).value, Some(false));
        assert_eq!(read_flag("k", &JsValue::Undefined), absent());
    }
}
