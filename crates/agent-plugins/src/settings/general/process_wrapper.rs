//! Port of `src/settings/general/process-wrapper.ts` at sha256:bc2a6eef7c93.
//!
//! The launcher of `processWrapper` and `CLAUDE_CODE_PROCESS_WRAPPER` (settings-reference.md,
//! `processWrapper`; corporate-launcher.md, "Format of the launcher value"). Both take the same
//! format: "Claude Code parses the value as an argument list, not a shell command". Takes the
//! host's `PolicyView`: stays TypeScript in the package (the module comment of `crate::settings`),
//! so it exports nothing.

use indexmap::IndexMap;
use serde::Serialize;

use crate::js::JsValue;
use crate::js::json_parse::json_parse;
use crate::js::{is_whitespace, trim};
use crate::settings::view::PolicyView;

const PROCESS_WRAPPER_ENV: &str = "CLAUDE_CODE_PROCESS_WRAPPER";

/// What a launcher value reads as.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum LauncherRead {
    /// The argument list.
    Argv {
        /// Its entries.
        argv: Vec<String>,
    },
    /// The value cannot be used: Claude Code "refuses to start the affected process" and reports
    /// this.
    Problem {
        /// Why.
        problem: String,
    },
}

fn fail(reason: &str) -> LauncherRead {
    LauncherRead::Problem {
        problem: format!("{PROCESS_WRAPPER_ENV}: {reason}"),
    }
}

/// A value that starts with `[` is "read as a JSON string array".
fn json_array(value: &str) -> LauncherRead {
    let Ok(parsed) = json_parse(value) else {
        return fail("the launcher is not a valid JSON array of strings");
    };
    let argv: Option<Vec<String>> = parsed.as_array().and_then(|items| {
        items
            .iter()
            .map(|one| one.as_str().map(str::to_owned))
            .collect()
    });
    match argv {
        Some(argv) if !argv.is_empty() => LauncherRead::Argv { argv },
        Some(_) | None => fail("the launcher is not a non-empty JSON array of strings"),
    }
}

/// The text without its double-quoted runs (`/"[^"]*"/g` replaced by nothing).
fn without_quoted(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(open) = rest.find('"') {
        let (before, quoted) = rest.split_at(open);
        let after_open = quoted.get(1..).unwrap_or_default();
        match after_open.find('"') {
            Some(close) => {
                out.push_str(before);
                rest = after_open.get(close + 1..).unwrap_or_default();
            }
            None => {
                out.push_str(before);
                out.push('"');
                rest = after_open;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Unquoted operators are rejected; quoted operators are ordinary argument text
/// (`/[;|&]|\$\(/`).
fn has_shell_operator(value: &str) -> bool {
    let unquoted = without_quoted(value);
    unquoted.contains(['|', ';', '&']) || unquoted.contains("$(")
}

/// The tokens of `/(?:[^\s"]|"[^"]*")+/g`: runs of non-blank characters and quoted runs.
fn token_runs(value: &str) -> Vec<String> {
    let chars: Vec<char> = value.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let mut token = String::new();
        while let Some(character) = chars.get(at) {
            if is_whitespace(*character) {
                break;
            }
            if *character == '"' {
                let close = chars
                    .iter()
                    .enumerate()
                    .skip(at + 1)
                    .find(|(_, other)| **other == '"')
                    .map(|(index, _)| index);
                let Some(close) = close else { break };
                token.extend(chars.iter().take(close + 1).skip(at));
                at = close + 1;
            } else {
                token.push(*character);
                at += 1;
            }
        }
        if token.is_empty() {
            at += 1;
        } else {
            tokens.push(token);
        }
    }
    tokens
}

/// Whitespace separates tokens, double quotes group; a single quote is an ordinary character.
fn tokens(value: &str) -> LauncherRead {
    if has_shell_operator(value) {
        return fail("the launcher holds a shell operator; write an argument list");
    }
    if !value.matches('"').count().is_multiple_of(2) {
        return fail("the launcher has an unterminated double quote");
    }
    let found: Vec<String> = token_runs(value)
        .into_iter()
        .map(|token| token.replace('"', ""))
        .collect();
    if found.is_empty() {
        fail("the launcher is empty")
    } else {
        LauncherRead::Argv { argv: found }
    }
}

/// The argv prefix a launcher value stands for, or why it cannot be used.
fn parse_launcher(value: &str) -> LauncherRead {
    let text = trim(value);
    if text.starts_with('[') {
        json_array(text)
    } else {
        tokens(text)
    }
}

/// The per-session launcher, ignored on Windows.
#[must_use]
pub fn read_process_wrapper(
    view: &dyn PolicyView,
    env: &IndexMap<String, String>,
    windows: bool,
) -> Option<LauncherRead> {
    if windows {
        return None;
    }
    let value = match env.get(PROCESS_WRAPPER_ENV) {
        Some(value) => Some(value.clone()),
        None => view
            .get("processWrapper")
            .and_then(|set| JsValue::as_str(&set).map(str::to_owned)),
    };
    value.as_deref().map(parse_launcher)
}
