//! Port of `src/hooks/input/build.ts`, switched.
//!
//! The JSON a command hook reads on stdin and an HTTP hook receives as its POST body, from the
//! event's facts and the session (hooks.md, "Hook input and output").

use serde::Serialize;

use super::common::{CommonInput, common_input};
use super::conditional_fields::apply_conditional_fields;
use super::facts::fact_schema_of;
use crate::format::issues::{PluginIssue, error, warning};
use crate::hooks::contracts::HookSession;
use crate::js::value::to_js_value;
use crate::js::{
    JsObject, JsValue, in_property_order, json_stringify, number_to_string, string_of,
};

/// What building an input gives.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BuiltInput {
    /// `undefined` when the facts do not make a valid input: the hook does not run. Left out then,
    /// as the wire leaves out a member set to `undefined`.
    #[serde(skip_serializing_if = "JsValue::is_undefined")]
    pub input: JsValue,
    /// Errors for facts that fail the official shape, warnings for what was dropped.
    pub issues: Vec<PluginIssue>,
}

/// `hooks.<Event>.input.<field>`, an array index as `[n]` as in the select stage and the warnings.
fn path_of(event: &str, segments: &[JsValue]) -> String {
    segments.iter().fold(
        format!("hooks.{event}.input"),
        |path, segment| match segment {
            JsValue::Number(index) => format!("{path}[{}]", number_to_string(*index)),
            other @ (JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::String(_)
            | JsValue::Array(_)
            | JsValue::Object(_)) => format!("{path}.{}", string_of(other)),
        },
    )
}

/// A field the host set to `undefined` is a field it did not set: JSON cannot carry it.
fn without_undefined(facts: &JsObject) -> JsObject {
    in_property_order(
        facts
            .iter()
            .filter(|(_, value)| !value.is_undefined())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

/// The common fields and the event's own facts as the input of `event`: `{ ...common, ...facts }`.
fn join_input(common: &CommonInput, facts: JsObject) -> JsValue {
    let mut joined = match to_js_value(common) {
        Ok(JsValue::Object(members)) => members,
        Ok(_) | Err(_) => JsObject::new(),
    };
    for (key, value) in facts {
        joined.insert(key, value);
    }
    JsValue::Object(in_property_order(joined))
}

/// The JSON a command hook reads on stdin and an HTTP hook receives as its POST body, from the
/// event's facts and the session (hooks.md, "Hook input and output"): the common input fields, then
/// the event's own. The facts are checked against the official input type of the event; a wrong
/// type or enum value is an error issue and no input, a field the event does not have or has only in
/// other cases (`conditional-fields.ts`) is dropped with a warning.
///
/// # Errors
///
/// An `event` that is no `HookEvent` has no schema: the TypeScript's type rules it out, and its
/// lookup would throw on `undefined`.
pub fn build_hook_input(
    event: &str,
    facts: &JsObject,
    session: &HookSession,
) -> Result<BuiltInput, String> {
    let Some(schema) = fact_schema_of(event) else {
        return Err(format!("No facts schema for the hook event \"{event}\""));
    };
    let given = without_undefined(facts);
    let parsed = match schema.safe_parse(&JsValue::Object(given.clone())) {
        Ok(data) => data.as_object().cloned().unwrap_or_default(),
        Err(found) => {
            return Ok(BuiltInput {
                input: JsValue::Undefined,
                issues: found
                    .issues
                    .iter()
                    .map(|issue| error(&path_of(event, issue.path()), issue.message()))
                    .collect(),
            });
        }
    };
    let mut issues: Vec<PluginIssue> = given
        .keys()
        .filter(|key| !parsed.contains_key(*key))
        .map(|key| {
            warning(
                &path_of(event, &[JsValue::String(key.clone())]),
                &format!("\"{key}\" is not a field of the {event} input and is dropped"),
            )
        })
        .collect();
    let refined = apply_conditional_fields(event, &parsed);
    let common = common_input(event, session);
    issues.extend(refined.issues);
    issues.extend(common.issues);
    // Facts come last: an event's own `agent_id` and `agent_type` (SubagentStart, SubagentStop) win.
    let input = join_input(&common.fields, refined.facts);
    Ok(BuiltInput { input, issues })
}

/// One JSON text for both transports: stdin of a command hook, body of an HTTP hook.
#[must_use]
pub fn serialize_hook_input(input: &JsValue) -> Option<String> {
    json_stringify(input)
}

crate::registry::exports! {
    "hooks/input/build" {
        "buildHookInput" => fallible |event: String, facts: JsObject, session: HookSession|
            build_hook_input(&event, &facts, &session);
        "serializeHookInput" => ok |input: JsValue| serialize_hook_input(&input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_names_an_index_in_brackets() {
        let segments = [
            JsValue::String("tool_calls".to_owned()),
            JsValue::Number(2.0),
            JsValue::String("tool_name".to_owned()),
        ];
        assert_eq!(
            path_of("PostToolBatch", &segments),
            "hooks.PostToolBatch.input.tool_calls[2].tool_name"
        );
    }

    #[test]
    fn an_event_with_no_schema_throws() {
        let built = build_hook_input("constructor", &JsObject::new(), &HookSession::default());
        assert!(built.is_err());
    }
}
