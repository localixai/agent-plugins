//! Port of `src/format/workflow-meta.ts`, switched.
//!
//! The `meta` block of a workflow script (workflows.md, "What the saved script looks like", "Edit a
//! saved script"; agent-sdk/typescript.md, "Workflow": the script "must begin with `export const meta
//! = { name, description }` as a literal"). Claude Code reads it without running the script, so the
//! kit does too: it finds the literal in the tokens, reads it, and rewrites single fields in place,
//! keeping every other byte.

use serde::{Deserialize, Serialize};

use super::issues::{PluginIssue, error, warning};
use super::js_literal::{LiteralObject, LiteralRead, read_object_literal};
use super::js_tokens::{JsToken, tokenize};
use crate::js::{JsObject, JsValue, in_property_order, trim};
use crate::protocol::is_record::is_record;

/// The `meta` of a workflow script.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowMeta {
    /// `name`, trimmed.
    pub name: String,
    /// `description`.
    pub description: String,
    /// The stage titles of `phases`, in order; `[]` when there are none.
    pub phases: Vec<String>,
    /// The entries behind `phases`, as written, so an object entry keeps its other fields
    /// (judgment): a title, or an object with a `title` and whatever else it carries.
    pub phase_entries: Vec<JsValue>,
    /// `whenToUse`, when it is a string (judgment: the docs do not name the field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    /// Every other literal field, as written.
    pub extra: JsObject,
}

/// A script split at the end of its `meta` statement.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptHead {
    /// The `meta`, when it reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<WorkflowMeta>,
    /// What is wrong with it.
    pub issues: Vec<PluginIssue>,
    /// The tokens of the whole script, or of its head up to the `meta` statement's end
    /// (`ScriptScope`).
    pub tokens: Vec<JsToken>,
    /// The literal, when the script begins with one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<LiteralObject>,
    /// Where the `meta` statement ends (after its `;`); the body starts here. `0` without a
    /// literal.
    pub head_end: usize,
    /// The index of the first token of the body.
    pub body_index: usize,
}

const CORE_KEYS: [&str; 3] = ["name", "description", "phases"];
const WHEN_TO_USE: &str = "whenToUse";
const OPENING: [&str; 4] = ["export", "const", "meta", "="];
const NO_LITERAL: &str = "must be a plain object literal with a name and a description";

fn nothing(issues: Vec<PluginIssue>, tokens: Vec<JsToken>) -> ScriptHead {
    ScriptHead {
        meta: None,
        issues,
        tokens,
        object: None,
        head_end: 0,
        body_index: 0,
    }
}

fn required(fields: &JsObject, key: &str) -> Result<String, PluginIssue> {
    match fields.get(key) {
        Some(JsValue::String(value)) if !trim(value).is_empty() => Ok(value.clone()),
        _ => Err(error(
            &format!("meta.{key}"),
            &format!("meta needs a {key}: a string that is not empty"),
        )),
    }
}

fn title_of(entry: &JsValue) -> Option<String> {
    if let JsValue::String(entry) = entry {
        return Some(entry.clone());
    }
    let title = if is_record(entry) {
        entry.get("title")
    } else {
        None
    };
    title.and_then(JsValue::as_str).map(str::to_owned)
}

fn as_entry(entry: &JsValue, title: &str) -> JsValue {
    if let JsValue::Object(members) = entry
        && is_record(entry)
    {
        let mut copy = members.clone();
        copy.insert("title".to_owned(), JsValue::from(title));
        return JsValue::Object(copy);
    }
    JsValue::from(title)
}

struct ReadPhases {
    titles: Vec<String>,
    entries: Vec<JsValue>,
    issues: Vec<PluginIssue>,
}

/// `phases`: a list of titles (or of objects with a `title`); what is not one is skipped with a
/// warning.
fn read_phases(value: Option<&JsValue>) -> ReadPhases {
    let none = || ReadPhases {
        titles: Vec::new(),
        entries: Vec::new(),
        issues: Vec::new(),
    };
    let Some(value) = value.filter(|value| **value != JsValue::Undefined) else {
        return none();
    };
    let JsValue::Array(items) = value else {
        let issue = warning(
            "meta.phases",
            "meta.phases must be a list of titles; ignored",
        );
        return ReadPhases {
            issues: vec![issue],
            ..none()
        };
    };
    let found: Vec<(String, JsValue)> = items
        .iter()
        .filter_map(|entry| title_of(entry).map(|title| (title.clone(), as_entry(entry, &title))))
        .collect();
    let skipped = items.len() - found.len();
    let issues = if skipped == 0 {
        Vec::new()
    } else {
        vec![warning(
            "meta.phases",
            &format!("meta.phases has {skipped} entries that are not a title; skipped"),
        )]
    };
    let (titles, entries) = found.into_iter().unzip();
    ReadPhases {
        titles,
        entries,
        issues,
    }
}

fn build_meta(fields: &JsObject) -> (Option<WorkflowMeta>, Vec<PluginIssue>) {
    let name = required(fields, "name");
    let description = required(fields, "description");
    let (name, description) = match (name, description) {
        (Ok(name), Ok(description)) => (name, description),
        (name, description) => {
            let missing = [name.err(), description.err()].into_iter().flatten();
            return (None, missing.collect());
        }
    };
    let phases = read_phases(fields.get("phases"));
    let when_to_use = fields
        .get(WHEN_TO_USE)
        .and_then(JsValue::as_str)
        .map(str::to_owned);
    let extra = fields
        .iter()
        .filter(|(key, _)| {
            !CORE_KEYS.contains(&key.as_str()) && !(*key == WHEN_TO_USE && when_to_use.is_some())
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let meta = WorkflowMeta {
        name: trim(&name).to_owned(),
        description,
        phases: phases.titles,
        phase_entries: phases.entries,
        when_to_use,
        extra,
    };
    (Some(meta), phases.issues)
}

/// How much of a script to tokenize: the `meta` statement only, or all of it for a scan of the
/// body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScriptScope {
    /// The `meta` statement only.
    Head,
    /// The whole script.
    Whole,
}

/// The head ends one token after the `}` that closes the literal (the token that may be its `;`),
/// or as soon as the first four tokens are not `export const meta =`.
fn head_end(tokens: &[JsToken], depth: i64) -> bool {
    let count = tokens.len();
    if count <= OPENING.len() {
        return !could_start_meta(tokens);
    }
    depth == 0
        && count > OPENING.len() + 1
        && count
            .checked_sub(2)
            .and_then(|at| tokens.get(at))
            .is_some_and(|token| token.text == "}")
}

fn starts_with_meta(tokens: &[JsToken]) -> bool {
    OPENING
        .iter()
        .enumerate()
        .all(|(index, word)| tokens.get(index).is_some_and(|token| token.text == *word))
}

/// Whether the tokens that came before a lexical problem still can be the start of `export const
/// meta =`.
fn could_start_meta(tokens: &[JsToken]) -> bool {
    OPENING
        .iter()
        .enumerate()
        .all(|(index, word)| tokens.get(index).is_none_or(|token| token.text == *word))
}

fn not_java_script(problem: &str) -> PluginIssue {
    error(
        "meta",
        &format!("the script is not valid JavaScript: {problem}"),
    )
}

/// The script does not begin with `export const meta =`: not the first statement, or a lexical
/// problem cut it short.
fn not_meta(tokens: Vec<JsToken>, problem: Option<&str>) -> ScriptHead {
    if let Some(problem) = problem
        && could_start_meta(&tokens)
    {
        return nothing(vec![not_java_script(problem)], tokens);
    }
    let message = "`export const meta = {…}` must be the first statement of the script";
    nothing(vec![error("meta", message)], tokens)
}

/// Splits a script at the end of its `meta` statement and reads the literal. What the docs leave
/// open (the severity of each finding, blank names) is in spec/judgments.md. A lexical problem (an
/// unterminated string, template or comment) is the runtime's to report when it lies after the
/// `meta` statement ("If the body has a syntax error, Claude Code reports it when you run the
/// workflow"): the `meta` is still read. When the literal cannot be read at all, the problem may
/// be what broke it, so it is the error (judgment).
#[must_use]
pub fn read_script_head(source: &str, scope: ScriptScope) -> ScriptHead {
    let mut stop = head_end;
    let read = match scope {
        ScriptScope::Head => tokenize(source, Some(&mut stop)),
        ScriptScope::Whole => tokenize(source, None),
    };
    let (tokens, problem) = (read.tokens, read.problem);
    if !starts_with_meta(&tokens) {
        return not_meta(tokens, problem.as_deref());
    }
    let object = match read_object_literal(&tokens, OPENING.len()) {
        LiteralRead::Found { found } => found,
        LiteralRead::Problem { problem: not_read } => {
            if let Some(problem) = problem {
                return nothing(vec![not_java_script(&problem)], tokens);
            }
            let message = format!(
                "meta {NO_LITERAL} ({not_read}); Claude Code drops /<name> from / autocomplete"
            );
            return nothing(vec![warning("meta", &message)], tokens);
        }
    };
    let fields: JsObject = in_property_order(
        object
            .properties
            .iter()
            .map(|each| (each.key.clone(), each.value.clone()))
            .collect(),
    );
    let (meta, issues) = build_meta(&fields);
    let semicolon = tokens
        .get(object.end_index)
        .filter(|token| token.text == ";")
        .map(|token| token.end);
    ScriptHead {
        meta,
        issues,
        head_end: semicolon.unwrap_or(object.end),
        body_index: object.end_index + usize::from(semicolon.is_some()),
        tokens,
        object: Some(object),
    }
}

/// The `meta` of a script and what is wrong with it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MetaRead {
    /// The `meta`, when it reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<WorkflowMeta>,
    /// What is wrong with it.
    pub issues: Vec<PluginIssue>,
}

/// The `meta` of a script and what is wrong with it. Never runs the script, never throws.
#[must_use]
pub fn read_workflow_meta(source: &str) -> MetaRead {
    let ScriptHead { meta, issues, .. } = read_script_head(source, ScriptScope::Head);
    MetaRead { meta, issues }
}

crate::registry::exports! {
    "format/workflow-meta" {
        "readScriptHead" => ok |source: String, scope: ScriptScope| read_script_head(&source, scope);
        "readWorkflowMeta" => ok |source: String| read_workflow_meta(&source);
    }
}
