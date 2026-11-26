//! Port of `src/agents/workflow/structured-output.ts`, switched.
//!
//! The `schema` option of an `agent()` call (workflows.md, "What the saved script looks like"): Claude
//! Code "checks the schema before starting the subagent: when it can prove the schema contradicts
//! itself, the call fails with an error naming the contradiction", and "if the subagent's output still
//! fails validation after five attempts, the call fails with an error that includes the last validation
//! failure"; `MAX_STRUCTURED_OUTPUT_RETRIES` changes the attempt count (env-vars.md). Pure decisions: the
//! kit validates no output and starts no agent.
//!
//! A pattern of `patternProperties` is run by `js::reg_exp`, which refuses some patterns
//! JavaScript runs (lookaround, backreferences, `\p{…}`): such a pattern is one that may match, as
//! one that does not compile is, so a key it would not have matched is not ruled out here.

use indexmap::IndexMap;
use serde::Serialize;

use crate::format::env_number::positive_whole_env;
use crate::format::issues::{PluginIssue, error};
use crate::js::reg_exp::unicode_reg_exp;
use crate::js::{JsObject, JsValue, number_to_string, property_order};

/// A pattern of `patternProperties` that matches `key`; one that is not a pattern matches nothing we can prove.
fn pattern_matches(pattern: &str, key: &str) -> Option<bool> {
    unicode_reg_exp(pattern)
        .ok()
        .map(|regex| regex.is_match(key))
}

/// `isRecord(value)`: an object that is no array and not `null`.
fn record_of(value: Option<&JsValue>) -> Option<&JsObject> {
    match value {
        Some(JsValue::Object(members)) => Some(members),
        _ => None,
    }
}

/// Whether `additionalProperties: false` rules `key` out: no `properties` entry, and no pattern that may match.
fn ruled_out(node: &JsObject, key: &str) -> bool {
    let named =
        record_of(node.get("properties")).is_some_and(|properties| properties.contains_key(key));
    if named {
        return false;
    }
    let patterns: Vec<&String> = record_of(node.get("patternProperties"))
        .map(|patterns| {
            property_order(patterns)
                .into_iter()
                .map(|(key, _)| key)
                .collect()
        })
        .unwrap_or_default();
    patterns
        .iter()
        .all(|pattern| pattern_matches(pattern, key) == Some(false))
}

const OBJECT_TYPE: &str = "object";

/// The schema describes an object: `type` is `object` (or a list of just that).
fn is_object_type(node: &JsObject) -> bool {
    match node.get("type") {
        None | Some(JsValue::Undefined) => false,
        Some(JsValue::Array(each)) => each.iter().all(|one| one.as_str() == Some(OBJECT_TYPE)),
        Some(one) => one.as_str() == Some(OBJECT_TYPE),
    }
}

/// A `required` key that `additionalProperties: false` rules out, in a schema that is an object (judgment: `type: 'object'`).
fn contradictions(node: &JsObject, path: &str) -> Vec<PluginIssue> {
    let required: &[JsValue] = match node.get("required") {
        Some(JsValue::Array(required)) => required,
        _ => &[],
    };
    if node.get("additionalProperties") != Some(&JsValue::Bool(false)) || !is_object_type(node) {
        return Vec::new();
    }
    required
        .iter()
        .filter_map(JsValue::as_str)
        .filter(|key| ruled_out(node, key))
        .map(|key| {
            error(
                path,
                &format!(
                    "schema contradicts itself: \"{key}\" is required, but additionalProperties is false and \"{key}\" is not a property"
                ),
            )
        })
        .collect()
}

/// The sub-schemas a contradiction is looked for in: each property and the items of an array.
fn children<'a>(node: &'a JsObject, path: &str) -> Vec<(String, &'a JsValue)> {
    let properties = record_of(node.get("properties"))
        .map(property_order)
        .unwrap_or_default();
    let items = node
        .get("items")
        .filter(|items| matches!(items, JsValue::Object(_)));
    properties
        .into_iter()
        .map(|(key, value)| (format!("{path}.properties.{key}"), value))
        .chain(items.map(|items| (format!("{path}.items"), items)))
        .collect()
}

fn walk(node: &JsValue, path: &str) -> Vec<PluginIssue> {
    let JsValue::Object(node) = node else {
        return Vec::new();
    };
    let mut issues = contradictions(node, path);
    for (at, child) in children(node, path) {
        issues.extend(walk(child, &at));
    }
    issues
}

/// The contradictions the kit can prove in a `schema` before the subagent starts, each an error that
/// names the key; none means the call goes on. Only the documented one is proven: a `required` key that
/// `additionalProperties: false` rules out, at the top or in a nested property or items schema.
#[must_use]
pub fn check_agent_schema(schema: &JsValue) -> Vec<PluginIssue> {
    walk(schema, "schema")
}

/// "Defaults to 5, a first attempt plus four retries".
const DEFAULT_STRUCTURED_OUTPUT_ATTEMPTS: f64 = 5.0;
const STRUCTURED_OUTPUT_ENV: &str = "MAX_STRUCTURED_OUTPUT_RETRIES";

/// The environment as the host gives it.
pub type Env = IndexMap<String, Option<String>>;

/// How many attempts a subagent's structured output gets: `MAX_STRUCTURED_OUTPUT_RETRIES` or 5. A value
/// that is not a positive whole number (`2e3`, `64_000` are: J3.17) is no value (judgment, as for the
/// stall window).
#[must_use]
pub fn structured_output_attempts(env: &Env) -> f64 {
    let value = env.get(STRUCTURED_OUTPUT_ENV).and_then(Option::as_deref);
    positive_whole_env(value).unwrap_or(DEFAULT_STRUCTURED_OUTPUT_ATTEMPTS)
}

/// What follows an attempt whose output failed validation: retry, or fail with the issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AfterInvalidOutput {
    /// Whether the call goes on.
    pub retry: bool,
    /// Why it fails, when it does not go on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<PluginIssue>,
}

/// What follows an attempt whose output failed validation: `failures` holds the validation failure of
/// every attempt so far, the last one being the attempt that just ended. The call goes on until the
/// attempts are used up, then fails with an error that includes the last failure (judgment: the wording
/// of the rest).
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "a count of failures is far below 2^53"
)]
pub fn after_invalid_output(failures: &[String], env: &Env) -> AfterInvalidOutput {
    let attempts = structured_output_attempts(env);
    if (failures.len() as f64) < attempts {
        return AfterInvalidOutput {
            retry: true,
            issue: None,
        };
    }
    let last = failures.last().map_or("no output", String::as_str);
    let message = format!(
        "agent output failed validation on all {} attempts; last failure: {last}",
        number_to_string(attempts)
    );
    AfterInvalidOutput {
        retry: false,
        issue: Some(error("agent", &message)),
    }
}

crate::registry::exports! {
    "agents/workflow/structured-output" {
        "checkAgentSchema" => ok |schema: JsValue| check_agent_schema(&schema);
        "structuredOutputAttempts" => ok |env: Env| structured_output_attempts(&env);
        "afterInvalidOutput" => ok |failures: Vec<String>, env: Env| after_invalid_output(&failures, &env);
    }
}
