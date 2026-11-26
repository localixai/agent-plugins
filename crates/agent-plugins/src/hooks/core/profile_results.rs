//! Port of `src/hooks/core/profile-results.ts`, switched.
//!
//! The portable view of a completed call (Agent Plugins Extended §18.2.7; core §7.3.7): the
//! pending profile of the call as it ran, with the result projected from Claude Code's
//! `tool_response` on `PostToolUse` or its `error` on `PostToolUseFailure`. Where the result cannot
//! be given (no exit code, no HTTP status, no `CallToolResult`, no text Claude received), the whole
//! profile is omitted, as core §7.3.7 requires (Judgments J18.16, J18.21).

use serde::Deserialize;

use super::profiles::{ProfileContext, pending_profile};
use super::types::CoreToolProfile;
use crate::js::{JsObject, JsValue, is_safe_integer, number};
use crate::protocol::is_record::is_record;

/// What the call came to: its response, or the failure text Claude received.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Completed {
    /// Whether it succeeded.
    pub ok: bool,
    /// Its response, when it did.
    #[serde(default)]
    pub response: JsValue,
    /// The failure text, when it did not.
    #[serde(default)]
    pub error: String,
}

type Result = Option<JsObject>;
type Projector = fn(&JsObject, &Completed, &ProfileContext) -> Result;

fn record(value: &JsValue) -> JsObject {
    match value {
        JsValue::Object(members) if is_record(value) => members.clone(),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Array(_)
        | JsValue::Object(_) => JsObject::new(),
    }
}

fn text(value: &JsValue) -> Option<&str> {
    value.as_str()
}

fn member<'a>(members: &'a JsObject, key: &str) -> &'a JsValue {
    members.get(key).unwrap_or(&JsValue::Undefined)
}

fn object(entries: impl IntoIterator<Item = (&'static str, JsValue)>) -> JsObject {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

fn string(value: &str) -> JsValue {
    JsValue::String(value.to_owned())
}

/// `^Exit code (\d+)\r?\n?`: the line and its digits.
fn exit_line(error: &str) -> Option<(usize, &str)> {
    let rest = error.strip_prefix("Exit code ")?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let after = rest.get(digits..)?;
    let after = after.strip_prefix('\r').unwrap_or(after);
    let after = after.strip_prefix('\n').unwrap_or(after);
    Some((error.len() - after.len(), rest.get(..digits)?))
}

/// A command with no exit code: run in the background or interrupted (Judgment J18.16).
fn no_exit_code(input: &JsObject, response: &JsObject) -> bool {
    member(input, "run_in_background").as_bool() == Some(true)
        || !member(response, "backgroundTaskId").is_undefined()
        || member(response, "interrupted").as_bool() == Some(true)
}

fn joined(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<&str>>()
        .join("\n")
}

/// J18.21: the exit code of a completed command when the host gives none. `PostToolUse` means the
/// command succeeded, so `0`, except where `returnCodeInterpretation` marks a non-error exit code
/// with special meaning: that code is unknown, and the profile is omitted.
fn success_code(response: &JsObject) -> Option<f64> {
    member(response, "returnCodeInterpretation")
        .is_undefined()
        .then_some(0.0)
}

/// `shell`: the output Claude received and the exit code the host observed.
fn shell(input: &JsObject, call: &Completed, context: &ProfileContext) -> Result {
    if call.ok {
        let response = record(&call.response);
        if no_exit_code(input, &response) {
            return None;
        }
        let exit = context
            .facts
            .exit_code
            .or_else(|| success_code(&response))?;
        let output = joined(&[
            text(member(&response, "stdout")),
            text(member(&response, "stderr")),
        ]);
        return Some(object([
            ("output", JsValue::String(output)),
            ("exit_code", JsValue::Number(exit)),
        ]));
    }
    if member(input, "run_in_background").as_bool() == Some(true) {
        return None;
    }
    let line = exit_line(&call.error);
    let exit = context
        .facts
        .exit_code
        .or_else(|| line.map(|(_, digits)| number(digits)))?;
    let output = match line {
        None => call.error.as_str(),
        Some((length, _)) => call.error.get(length..).unwrap_or_default(),
    };
    Some(object([
        ("output", string(output)),
        ("exit_code", JsValue::Number(exit)),
    ]))
}

fn binary(media_type: &JsValue, data: &JsValue) -> Option<Vec<JsValue>> {
    let (media_type, data) = (text(media_type)?, text(data)?);
    Some(vec![JsValue::Object(object([
        ("type", string("binary")),
        ("media_type", string(media_type)),
        ("data", string(data)),
    ]))])
}

type ReadKind = fn(&JsObject, &JsObject, &ProfileContext) -> Option<Vec<JsValue>>;

/// The text Claude received from the call, which only the host has: byte for byte the call's
/// `tool_result` string (J18.16). The docs fix no format for its line numbers, so it is never
/// rebuilt from `file.content`; without it the profile is omitted.
fn received_text(
    _file: &JsObject,
    _response: &JsObject,
    context: &ProfileContext,
) -> Option<Vec<JsValue>> {
    let read = context.facts.read_text.as_deref()?;
    Some(vec![JsValue::Object(object([
        ("type", string("text")),
        ("text", string(read)),
    ]))])
}

fn pages(_file: &JsObject, response: &JsObject, _context: &ProfileContext) -> Option<Vec<JsValue>> {
    let pages = member(response, "pages").as_array()?;
    Some(
        pages
            .iter()
            .flat_map(|page| {
                let page = record(page);
                binary(member(&page, "mediaType"), member(&page, "base64")).unwrap_or_default()
            })
            .collect(),
    )
}

/// One content list per `FileReadOutput` variant (sdk-tools.d.ts 0.3.321).
const READ_KINDS: [(&str, ReadKind); 5] = [
    ("text", received_text),
    ("image", |file, _, _| {
        binary(member(file, "type"), member(file, "base64"))
    }),
    ("pdf", |file, _, _| {
        binary(&string("application/pdf"), member(file, "base64"))
    }),
    ("parts", pages),
    ("notebook", received_text),
];

fn file_read(_input: &JsObject, call: &Completed, context: &ProfileContext) -> Result {
    if !call.ok {
        return Some(object([("error", string(&call.error))]));
    }
    let response = record(&call.response);
    let kind = text(member(&response, "type"))?;
    let read = READ_KINDS.iter().find(|(name, _)| *name == kind)?.1;
    let content = read(&record(member(&response, "file")), &response, context)?;
    Some(object([("content", JsValue::Array(content))]))
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a projector answers `Option`: it is stored in a table of function pointers"
)]
fn file_modify(_input: &JsObject, call: &Completed, _context: &ProfileContext) -> Result {
    if call.ok {
        Some(JsObject::new())
    } else {
        Some(object([("error", string(&call.error))]))
    }
}

/// `web.fetch`: only a fetch with an HTTP response has a profile.
fn web_fetch(_input: &JsObject, call: &Completed, _context: &ProfileContext) -> Result {
    if !call.ok {
        return None;
    }
    let response = record(&call.response);
    let code = member(&response, "code").as_f64()?;
    if !is_safe_integer(code) || code.fract() != 0.0 || !(100.0..=599.0).contains(&code) {
        return None;
    }
    let result = text(member(&response, "result")).unwrap_or_default();
    Some(object([
        ("status_code", JsValue::Number(code)),
        (
            "content",
            JsValue::Array(vec![JsValue::Object(object([
                ("type", string("text")),
                ("text", string(result)),
            ]))]),
        ),
    ]))
}

/// One `{ title, url, snippet? }` per source; a string entry is commentary and gives none.
fn search_items(results: &JsValue) -> Vec<JsValue> {
    let Some(results) = results.as_array() else {
        return Vec::new();
    };
    results
        .iter()
        .flat_map(|entry| {
            let sources = entry
                .as_object()
                .filter(|_| is_record(entry))
                .and_then(|members| member(members, "content").as_array());
            sources.into_iter().flatten().filter_map(|source| {
                let source = record(source);
                let (title, url) = (
                    text(member(&source, "title"))?,
                    text(member(&source, "url"))?,
                );
                if url.is_empty() {
                    return None;
                }
                let mut item = object([("title", string(title)), ("url", string(url))]);
                if let Some(snippet) = text(member(&source, "snippet")) {
                    item.insert("snippet".to_owned(), string(snippet));
                }
                Some(JsValue::Object(item))
            })
        })
        .collect()
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a projector answers `Option`: it is stored in a table of function pointers"
)]
fn web_search(_input: &JsObject, call: &Completed, _context: &ProfileContext) -> Result {
    if call.ok {
        let items = search_items(member(&record(&call.response), "results"));
        Some(object([("items", JsValue::Array(items))]))
    } else {
        Some(object([("error", string(&call.error))]))
    }
}

/// `mcp`: the server's own `CallToolResult`, which only the host has (`McpOutput` is processed).
fn mcp(_input: &JsObject, _call: &Completed, context: &ProfileContext) -> Result {
    context.facts.mcp_result.clone()
}

const PROJECTORS: [(&str, Projector); 6] = [
    ("shell", shell),
    ("file.read", file_read),
    ("file.modify", file_modify),
    ("web.fetch", web_fetch),
    ("web.search", web_search),
    ("mcp", mcp),
];

/// Tools whose pending profile has no completed form: every `Monitor` result.
const NO_RESULT: [&str; 1] = ["Monitor"];

/// The profile of a completed call, or `None` when none applies or its result is unknown.
#[must_use]
pub fn completed_profile(
    tool_name: &str,
    tool_input: &JsValue,
    call: &Completed,
    context: &ProfileContext,
) -> Option<CoreToolProfile> {
    if NO_RESULT.contains(&tool_name) {
        return None;
    }
    let pending = pending_profile(tool_name, tool_input, context)?;
    let project = PROJECTORS
        .iter()
        .find(|(name, _)| *name == pending.name)
        .map(|(_, project)| *project)?;
    let result = project(&record(tool_input), call, context)?;
    Some(CoreToolProfile {
        result: Some(result),
        ..pending
    })
}

crate::registry::exports! {
    "hooks/core/profile-results" {
        "completedProfile" => ok |tool_name: String, tool_input: JsValue, call: Completed, context: ProfileContext| completed_profile(&tool_name, &tool_input, &call, &context);
    }
}
