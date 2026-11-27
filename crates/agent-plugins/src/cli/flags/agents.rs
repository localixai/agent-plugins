//! Port of `src/cli/flags/agents.ts`, switched.
//!
//! `--agent` and `--agents` (cli-reference.md, "CLI flags"; sub-agents.md, "Choose the subagent
//! scope" and "Invoke subagents explicitly"; errors.md, "Invalid `--agents` configuration").
//! `--agent` names the session's agent and "overrides the `agent` setting". `--agents` is JSON of
//! definitions keyed by name; "with `--print`, the value can instead be the path to a JSON file";
//! Claude Code "validates the value at startup and exits on an invalid one", running the checks "in
//! order and stops at the first one that fails": the JSON, the definitions, the names.
//! `--safe-mode` and `CLAUDE_CODE_SAFE_MODE` "ignore `--agents` entirely"; with `--resume` or
//! `--continue` "an inline JSON value isn't checked and the session starts; a value read from a
//! file is checked on every launch". The refusal wording is errors.md's; what the docs leave open
//! is in spec/judgments.md.

use super::sideload::sideload_refusal;
use super::types::{Contribution, FlagContext, refuse};
use super::values::{Parsed, parse_json};
use crate::format::env_flag::is_truthy_env;
use crate::format::read_json::JsonRead;
use crate::js::{JsObject, JsValue, in_property_order, property_order, trim, trim_start};

/// The contribution of `--agent`.
#[must_use]
pub fn agent(name: &str) -> Contribution {
    Contribution {
        agent: Some(name.to_owned()),
        ..Contribution::default()
    }
}

/// "Specify an agent for the current session (overrides the `agent` setting)": the flag wins when
/// both name one.
#[must_use]
pub fn session_agent(flag: Option<String>, setting: Option<String>) -> Option<String> {
    flag.or(setting)
}

const HEADING: &str = "Error: Invalid --agents configuration:";
const MAX_PROBLEM_LINES: usize = 20;

/// "`color` and `experimental` aren't accepted here and are ignored rather than rejected."
const IGNORED_FIELDS: [&str; 2] = ["color", "experimental"];

/// The problems of one definition: `description` and `prompt` are the two a definition always
/// carries; `prompt` may be empty.
fn definition_problems(name: &str, definition: &JsValue) -> Vec<String> {
    if !definition.is_object() {
        return vec![format!("{name}: must be an object")];
    }
    let mut problems = Vec::new();
    let description = definition.get("description").and_then(JsValue::as_str);
    if description.is_none_or(|text| trim(text).is_empty()) {
        problems.push(format!("{name}.description: Required"));
    }
    if !definition.get("prompt").is_some_and(JsValue::is_string) {
        problems.push(format!("{name}.prompt: Required"));
    }
    problems
}

fn invalid(lines: &[String]) -> Contribution {
    let shown = lines.get(..MAX_PROBLEM_LINES).unwrap_or(lines);
    let rest = lines.len().saturating_sub(shown.len());
    let mut all = vec![HEADING.to_owned()];
    all.extend(shown.iter().cloned());
    if rest > 0 {
        all.push(format!("…and {rest} more"));
    }
    refuse(vec![all.join("\n")])
}

/// The checks in the order the docs give: schema, then names.
fn check_definitions(value: &JsValue) -> Contribution {
    let Some(members) = value.as_object() else {
        return invalid(&["the value must be a JSON object keyed by agent name".to_owned()]);
    };
    let entries = property_order(members);
    let schema: Vec<String> = entries
        .iter()
        .flat_map(|(name, definition)| definition_problems(name, definition))
        .collect();
    if !schema.is_empty() {
        return invalid(&schema);
    }
    let names: Vec<String> = entries
        .iter()
        .filter(|(name, _)| name.starts_with('-'))
        .map(|(name, _)| format!("{name}: agent names must not start with '-'"))
        .collect();
    if !names.is_empty() {
        return invalid(&names);
    }
    let kept: JsObject = entries
        .into_iter()
        .map(|(name, definition)| (name.clone(), without_ignored(definition)))
        .collect();
    Contribution {
        cli_agents: Some(in_property_order(kept)),
        ..Contribution::default()
    }
}

fn without_ignored(definition: &JsValue) -> JsValue {
    definition.as_object().map_or_else(
        || definition.clone(),
        |members| {
            let kept: JsObject = property_order(members)
                .into_iter()
                .filter(|(key, _)| !IGNORED_FIELDS.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            JsValue::Object(in_property_order(kept))
        },
    )
}

fn is_inline(value: &str) -> bool {
    trim_start(value).starts_with('{')
}

/// `--safe-mode` or `CLAUDE_CODE_SAFE_MODE=1` ("Equivalent to passing `--safe-mode`",
/// env-vars.md).
fn safe_mode_on(context: &FlagContext) -> bool {
    context.given.get("--safe-mode") == Some(&JsValue::Bool(true))
        || is_truthy_env(
            context
                .facts
                .env
                .get("CLAUDE_CODE_SAFE_MODE")
                .and_then(Option::as_deref),
        )
}

/// The value as JSON, or the refusal: an object starts with `{`; any other value is a path to a
/// file.
fn agents_json(value: &str, context: &FlagContext) -> Result<Parsed, String> {
    if is_inline(value) {
        return Ok(parse_json(value));
    }
    if !context.facts.print {
        return Err(
            "Error: --agents takes a JSON object, or a file path only with --print (-p)".to_owned(),
        );
    }
    let Some(file) = context.facts.files.get(value) else {
        return Err(format!("Error: --agents file not found: {value}"));
    };
    Ok(parse_json(&file.text))
}

fn validated(value: &str, context: &FlagContext) -> Contribution {
    match agents_json(value, context) {
        Err(refusal) => refuse(vec![refusal]),
        Ok(JsonRead::Problem { problem, .. }) => invalid(&[format!("invalid JSON: {problem}")]),
        Ok(JsonRead::Value { value }) => check_definitions(&value),
    }
}

/// An inline value on a resumed session is not checked: it defines agents when valid and is
/// ignored when not (judgment).
fn unchecked(value: &str, context: &FlagContext) -> bool {
    is_inline(value) && (context.facts.resume || context.facts.continuing)
}

/// The contribution of `--agents`.
#[must_use]
pub fn agents(value: &str, context: &FlagContext) -> Contribution {
    if let Some(refusal) = sideload_refusal("--agents", &context.facts) {
        return refuse(vec![refusal]);
    }
    if safe_mode_on(context) {
        return Contribution::default();
    }
    let result = validated(value, context);
    let failed = result
        .refusals
        .as_ref()
        .is_some_and(|lines| !lines.is_empty());
    if failed && unchecked(value, context) {
        Contribution::default()
    } else {
        result
    }
}

crate::registry::exports! {
    "cli/flags/agents" {
        "sessionAgent" => ok |flag: Option<String>, setting: Option<String>| session_agent(flag, setting);
    }
}
