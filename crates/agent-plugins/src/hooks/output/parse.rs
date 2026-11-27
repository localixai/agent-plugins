//! Port of `src/hooks/output/parse.ts`, switched.
//!
//! Reading one JSON value as the output of a hook (hooks.md, "JSON output", "Exit code 0"): "JSON
//! that can't be parsed or fails schema validation" is a non-blocking error, and the message names
//! what is wrong so the author can fix it. An unknown key is dropped; a value of the wrong type fails.
//!
//! A `CheckedOutput` is the value the schema answered, a [`JsValue`] object (`schemas.rs`). A name
//! that is no event throws what the TypeScript threw (`output_schema`).

use serde::{Deserialize, Serialize};

use crate::hooks::output::fields::has_text;
use crate::hooks::output::schemas::output_schema;
use crate::js::{JsValue, number_to_string, property_order, string_of};

/// What checking one value gives: the checked output, or why it is not one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckedResult {
    /// The output, its known fields only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<JsValue>,
    /// Why the value is not a valid output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
}

/// The output stage's reading of one JSON value (`ParseResult` of `verdict/types.ts`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    /// The checked output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<JsValue>,
    /// Why the value is not a valid output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    /// Keys of a valid output the schema does not name, ignored (`hookSpecificOutput.<key>` inside it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unrecognized_keys: Option<Vec<String>>,
}

/// Events whose `block` needs its explanation: "`reason` | Required when `decision` is `\"block\"`".
const REASON_REQUIRED: &[&str] = &["Stop", "SubagentStop"];

fn path_of(segments: &[JsValue]) -> String {
    segments.iter().fold(String::new(), |path, segment| {
        if let Some(index) = segment.as_f64() {
            format!("{path}[{}]", number_to_string(index))
        } else if path.is_empty() {
            string_of(segment)
        } else {
            format!("{path}.{}", string_of(segment))
        }
    })
}

fn missing_reason(event: &str, data: &JsValue) -> bool {
    REASON_REQUIRED.contains(&event)
        && data.get("decision").and_then(JsValue::as_str) == Some(BLOCK)
        && !has_text(data.get("reason").and_then(JsValue::as_str))
}

const BLOCK: &str = "block";

/// Checks `json` against `event`'s output schema.
///
/// # Errors
///
/// For a name that is no hook event, with what the TypeScript threw.
pub fn check_output(event: &str, json: &JsValue) -> Result<CheckedResult, String> {
    let parsed = output_schema(event)?.safe_parse(json);
    let data = match parsed {
        Ok(data) => data,
        Err(parsed) => {
            let found: Vec<String> = parsed
                .issues
                .iter()
                .map(|issue| {
                    let place = path_of(issue.path());
                    if place.is_empty() {
                        issue.message().to_owned()
                    } else {
                        format!("{place}: {}", issue.message())
                    }
                })
                .collect();
            return Ok(CheckedResult {
                data: None,
                failure: Some(found.join("; ")),
            });
        }
    };
    if missing_reason(event, &data) {
        return Ok(CheckedResult {
            data: None,
            failure: Some(format!(
                "reason: Required when decision is \"{BLOCK}\" on {event}"
            )),
        });
    }
    Ok(CheckedResult {
        data: Some(data),
        failure: None,
    })
}

fn keys_of(value: Option<&JsValue>) -> Vec<String> {
    value
        .and_then(JsValue::as_object)
        .map(|members| {
            property_order(members)
                .into_iter()
                .map(|(key, _)| key.clone())
                .collect()
        })
        .unwrap_or_default()
}

const SPECIFIC: &str = "hookSpecificOutput";

/// The keys of `json` the schema dropped: at the top level, and inside a `hookSpecificOutput` the
/// schema kept (named `hookSpecificOutput.<key>`). One that is gone whole is one key.
fn unrecognized_keys(json: &JsValue, data: &JsValue) -> Vec<String> {
    let kept = keys_of(Some(data));
    let top: Vec<String> = keys_of(Some(json))
        .into_iter()
        .filter(|key| !kept.contains(key))
        .collect();
    let in_json = json
        .as_object()
        .is_some_and(|members| members.contains_key(SPECIFIC));
    if !kept.iter().any(|key| key == SPECIFIC) || !in_json {
        return top;
    }
    let kept_inside = keys_of(data.get(SPECIFIC));
    let inside = keys_of(json.get(SPECIFIC))
        .into_iter()
        .filter(|key| !kept_inside.contains(key))
        .map(|key| format!("{SPECIFIC}.{key}"));
    top.into_iter().chain(inside).collect()
}

/// The output stage's `parse`: the checked output, or why it failed. A key the schema does not name
/// is ignored and listed in `unrecognizedKeys`, for the debug line `Hook JSON output had
/// unrecognized keys` (Agent Plugins Extended §7.4.5.1 item 1, Judgment J7.30).
///
/// # Errors
///
/// For a name that is no hook event, with what the TypeScript threw.
pub fn parse_hook_output(event: &str, json: &JsValue) -> Result<ParseResult, String> {
    let checked = check_output(event, json)?;
    let Some(data) = checked.data else {
        return Ok(ParseResult {
            output: None,
            failure: Some(
                checked
                    .failure
                    .unwrap_or_else(|| "the output is not valid".to_owned()),
            ),
            unrecognized_keys: None,
        });
    };
    let unknown = unrecognized_keys(json, &data);
    Ok(ParseResult {
        output: Some(data),
        failure: None,
        unrecognized_keys: (!unknown.is_empty()).then_some(unknown),
    })
}

crate::registry::exports! {
    "hooks/output/parse" {
        "checkOutput" => fallible |event: String, json: JsValue| check_output(&event, &json);
        "parseHookOutput" => fallible |event: String, json: JsValue| parse_hook_output(&event, &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    fn parsed(event: &str, text: &str) -> Option<String> {
        let json = json_parse(text).ok()?;
        match parse_hook_output(event, &json) {
            Ok(result) => serde_json::to_string(&result).ok(),
            Err(message) => Some(format!("throws {message}")),
        }
    }

    // The TypeScript before the switch, `parseHookOutput(event, json)` printed with `JSON.stringify`.
    #[test]
    fn outputs_are_read_as_the_typescript_read_them() {
        let cases = [
            (
                "Stop",
                r#"{"decision":"block","x":1,"hookSpecificOutput":{"hookEventName":"Stop","y":2,"additionalContext":3}}"#,
                r#"{"failure":"hookSpecificOutput.additionalContext: Invalid input: expected string, received number"}"#,
            ),
            (
                "PermissionRequest",
                r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow","updatedPermissions":[{"type":"setMode","mode":"manual","destination":"session"},{"type":"setMode","mode":"bad","destination":"x"}]}}}"#,
                r#"{"failure":"hookSpecificOutput.decision.updatedPermissions[1].mode: \"bad\" is not a permission mode; hookSpecificOutput.decision.updatedPermissions[1].destination: Invalid option: expected one of \"session\"|\"localSettings\"|\"projectSettings\"|\"userSettings\"|\"cliArg\""}"#,
            ),
            (
                "Nope",
                "{}",
                "throws undefined is not an object (evaluating 'SPECIFIC_SCHEMAS[event].exactOptional')",
            ),
        ];
        for (event, json, want) in cases {
            assert_eq!(parsed(event, json).as_deref(), Some(want), "{event} {json}");
        }
    }
}
