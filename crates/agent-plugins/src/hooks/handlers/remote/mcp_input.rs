//! Port of `src/hooks/handlers/remote/mcp-input.ts`, switched.
//!
//! The arguments of an `mcp_tool` hook (hooks.md, "MCP tool hook fields"): `input` holds "arguments
//! passed to the tool. String values support `${path}` substitution from the hook's JSON input,
//! such as `"${tool_input.file_path}"`".
//!
//! The page names only that form, so the rest is the kit's reading (spec/judgments.md): a path is
//! keys joined by dots, and a number is an array index; strings nested in objects and arrays are
//! substituted too; the text a path brings in is not read again; a value that is not a string is
//! written as text (an object as JSON); a path with no value is an empty string and a warning.
//! Numbers, booleans and null stay as they are, and so do keys.

use std::sync::LazyLock;

use indexmap::IndexSet;
use regex::Regex;
use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::js::json_parse::json_parse;
use crate::js::value::JsObject;
use crate::js::{JsValue, Pattern, number, property_order};

/// `${a.b_c.0}`: text that is not a path (`${not a path}`, `${}`) is not a reference.
static REFERENCE: Pattern =
    LazyLock::new(|| Regex::new(r"\$\{([A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)*)\}").ok());

/// `items[number]`: only a whole number inside the array names an item.
fn item_at(items: &[JsValue], index: f64) -> Option<&JsValue> {
    if !index.is_finite() || index < 0.0 || index.fract() != 0.0 {
        return None;
    }
    items
        .iter()
        .enumerate()
        .find(|(position, _)| (*position as f64) == index)
        .map(|(_, item)| item)
}

fn value_at<'a>(root: &'a JsValue, path: &str) -> Option<&'a JsValue> {
    let mut current = root;
    for key in path.split('.') {
        current = match current {
            JsValue::Array(items) => item_at(items, number(key))?,
            JsValue::Object(members) => members.get(key)?,
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::String(_) => return None,
        };
    }
    Some(current)
}

fn as_text(value: &JsValue) -> String {
    match value {
        JsValue::String(text) => text.clone(),
        JsValue::Null | JsValue::Undefined => String::new(),
        JsValue::Bool(_) | JsValue::Number(_) | JsValue::Array(_) | JsValue::Object(_) => {
            value.to_json_text()
        }
    }
}

/// The arguments to send, and a warning for each path that had no value.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct McpArguments {
    /// The arguments.
    pub input: JsObject,
    /// A warning for each path with no value.
    pub issues: Vec<PluginIssue>,
}

fn substitute_text(template: &str, hook_input: &JsValue, missing: &mut IndexSet<String>) -> String {
    let Some(reference) = REFERENCE.as_ref() else {
        return template.to_owned();
    };
    reference
        .replace_all(template, |captures: &regex::Captures<'_>| {
            let path = captures.get(1).map_or("", |group| group.as_str());
            match value_at(hook_input, path) {
                Some(JsValue::Undefined) | None => {
                    missing.insert(path.to_owned());
                    String::new()
                }
                Some(value) => as_text(value),
            }
        })
        .into_owned()
}

fn walk(value: &JsValue, hook_input: &JsValue, missing: &mut IndexSet<String>) -> JsValue {
    match value {
        JsValue::String(text) => JsValue::String(substitute_text(text, hook_input, missing)),
        JsValue::Array(items) => JsValue::Array(
            items
                .iter()
                .map(|item| walk(item, hook_input, missing))
                .collect(),
        ),
        JsValue::Object(members) => JsValue::Object(
            property_order(members)
                .into_iter()
                .map(|(key, inner)| (key.clone(), walk(inner, hook_input, missing)))
                .collect(),
        ),
        JsValue::Undefined | JsValue::Null | JsValue::Bool(_) | JsValue::Number(_) => value.clone(),
    }
}

/// `input` with every string's references read from `hook_input_json`, the input the hook receives.
#[must_use]
pub fn substitute_input(
    input: Option<&JsObject>,
    hook_input_json: &str,
    issue_path: &str,
) -> McpArguments {
    let hook_input = parse(hook_input_json);
    let mut missing = IndexSet::new();
    let substituted: JsObject = input
        .map(|members| {
            property_order(members)
                .into_iter()
                .map(|(key, value)| (key.clone(), walk(value, &hook_input, &mut missing)))
                .collect()
        })
        .unwrap_or_default();
    let issues = missing
        .iter()
        .map(|path| {
            warning(
                issue_path,
                &format!(
                    "${{{path}}} is not in the hook input and is replaced with an empty string."
                ),
            )
        })
        .collect();
    McpArguments {
        input: substituted,
        issues,
    }
}

fn parse(json: &str) -> JsValue {
    json_parse(json).unwrap_or(JsValue::Undefined)
}

crate::registry::exports! {
    "hooks/handlers/remote/mcp-input" {
        "substituteInput" => ok |input: Option<JsObject>, hook_input_json: String, issue_path: String| substitute_input(input.as_ref(), &hook_input_json, &issue_path);
    }
}
