//! Port of `src/hooks/verdict/stdout-kind.ts`, switched.
//!
//! What a hook's stdout is (hooks.md, "Exit code 0"): "For a hook that isn't async, your stdout
//! counts as JSON output only when you print one JSON object, on one line or several, with nothing
//! around it but whitespace. Claude Code reads any other stdout as plain text or a parse failure."

use serde::Serialize;

use crate::js::json_parse::json_parse;
use crate::js::{JsValue, trim};
use crate::protocol::is_record::is_record;

/// What stdout reads as.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StdoutReading {
    /// The stdout without the whitespace around it.
    pub text: String,
    /// One JSON object, to be checked against the output schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<JsValue>,
    /// Why it is a parse failure: it looks like JSON and is not one object. Neither `json` nor this:
    /// plain text or nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_failure: Option<String>,
}

fn try_parse(text: &str) -> Result<JsValue, String> {
    json_parse(text)
}

fn lines(text: &str) -> Vec<&str> {
    text.split('\n')
        .filter(|line| !trim(line).is_empty())
        .collect()
}

/// "A JSON output object that sets a field": judgment (spec/judgments.md), an object with a key.
fn sets_a_field(parsed: &Result<JsValue, String>) -> bool {
    parsed.as_ref().is_ok_and(|value| {
        is_record(value) && value.as_object().is_some_and(|keys| !keys.is_empty())
    })
}

const SEVERAL_OBJECTS: &str = "stdout holds several JSON values and one of them is a JSON output object; print that object with nothing else on stdout";

/// Reads a command hook's stdout.
#[must_use]
pub fn read_stdout(stdout: &str) -> StdoutReading {
    let text = trim(stdout).to_owned();
    let plain = |text: String| StdoutReading {
        text,
        json: None,
        parse_failure: None,
    };
    // "Output that doesn't start with `{`, or that starts with `{` and doesn't end with `}`: plain text.
    // A JSON array and a quoted JSON string are plain text by this rule."
    if !text.starts_with('{') || !text.ends_with('}') {
        return plain(text);
    }
    let whole = try_parse(&text);
    if let Ok(value) = &whole
        && is_record(value)
    {
        return StdoutReading {
            text,
            json: Some(value.clone()),
            parse_failure: None,
        };
    }
    let each: Vec<Result<JsValue, String>> = lines(&text)
        .into_iter()
        .map(|line| try_parse(trim(line)))
        .collect();
    // "Two or more lines that each parse as JSON on their own ...: plain text when no line is a JSON
    // output object that sets a field. When one line is, the output is a parse failure."
    if each.len() >= 2 && each.iter().all(Result::is_ok) {
        return if each.iter().any(sets_a_field) {
            StdoutReading {
                text,
                json: None,
                parse_failure: Some(SEVERAL_OBJECTS.to_owned()),
            }
        } else {
            plain(text)
        };
    }
    // "Anything else that starts with `{` and ends with `}` but isn't valid JSON: a parse failure."
    let parse_failure = match whole {
        Ok(_) => SEVERAL_OBJECTS.to_owned(),
        Err(message) => message,
    };
    StdoutReading {
        text,
        json: None,
        parse_failure: Some(parse_failure),
    }
}

/// An `http` hook's body (hooks.md, "HTTP response handling"): "2xx with an empty body: success";
/// "2xx with a JSON object body: parsed using the same JSON output schema"; "2xx with any other body,
/// such as plain text: non-blocking error".
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyReading {
    /// The body holds nothing.
    pub empty: bool,
    /// The JSON object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<JsValue>,
    /// Text that is not a JSON object (plain text, an array, a quoted string, bad JSON).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_object: Option<String>,
}

/// Reads an `http` hook's body.
#[must_use]
pub fn read_body(body: &str) -> BodyReading {
    let text = trim(body);
    if text.is_empty() {
        return BodyReading {
            empty: true,
            json: None,
            not_object: None,
        };
    }
    match try_parse(text) {
        Ok(value) if is_record(&value) => BodyReading {
            empty: false,
            json: Some(value),
            not_object: None,
        },
        Ok(_) => BodyReading {
            empty: false,
            json: None,
            not_object: Some("the body is not a JSON object".to_owned()),
        },
        Err(message) => BodyReading {
            empty: false,
            json: None,
            not_object: Some(message),
        },
    }
}

/// An async hook's response (hooks.md, "How async hooks execute"): "Alone on stdout: when the
/// response is the only text on stdout, it can span several lines" or "On one line of its own: an
/// async hook can print other text to stdout when the response fits on one line by itself". Judgment
/// (spec/judgments.md): when several lines are objects, the last one is the response.
#[must_use]
pub fn read_async_response(stdout: &str) -> Option<JsValue> {
    if let Some(alone) = read_stdout(stdout).json {
        return Some(alone);
    }
    lines(stdout)
        .into_iter()
        .filter_map(|line| try_parse(trim(line)).ok())
        .rfind(is_record)
}

crate::registry::exports! {
    "hooks/verdict/stdout-kind" {
        "readStdout" => ok |stdout: String| read_stdout(&stdout);
        "readBody" => ok |body: String| read_body(&body);
        "readAsyncResponse" => ok |stdout: String| read_async_response(&stdout);
    }
}
