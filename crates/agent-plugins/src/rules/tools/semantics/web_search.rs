//! Port of `src/rules/tools/semantics/web-search.ts`, switched.

use crate::format::issues::{PluginIssue, error};
use crate::js::JsValue;

/// An `allowed_domains` or `blocked_domains` value that holds at least one entry.
fn is_filled(value: Option<&JsValue>) -> bool {
    value
        .and_then(JsValue::as_array)
        .is_some_and(|items| !items.is_empty())
}

/// "Claude can scope results with `allowed_domains` to include only certain hosts, or
/// `blocked_domains` to exclude them. The two lists can't be combined in a single call."
/// (tools-reference.md, "WebSearch tool behavior"). A call with both lists filled is one error at
/// `tool_input`. An empty list is no list, a value that is not an array is left to `parseToolInput`
/// and the wording is a judgment (spec/judgments.md). How an entry matches a result's host is not in
/// the docs and is not modelled.
#[must_use]
pub fn validate_web_search_domains(input: &JsValue) -> Vec<PluginIssue> {
    let Some(members) = input.as_object() else {
        return Vec::new();
    };
    if is_filled(members.get("allowed_domains")) && is_filled(members.get("blocked_domains")) {
        vec![error(
            "tool_input",
            "allowed_domains and blocked_domains cannot be combined in a single call",
        )]
    } else {
        Vec::new()
    }
}

crate::registry::exports! {
    "rules/tools/semantics/web-search" {
        "validateWebSearchDomains" => ok |input: JsValue| validate_web_search_domains(&input);
    }
}
