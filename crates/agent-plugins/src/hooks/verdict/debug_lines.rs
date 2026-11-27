//! Ported from `src/hooks/verdict/debug-lines.ts`; only the core uses it now, so it has no wrapper.
//!
//! The debug-log lines the docs quote for how a hook's output was read, as warnings of the run: the
//! kit writes no log, the host does (judgment, spec/judgments.md; Agent Plugins Extended §7.4.12).
//!
//! `Subject`, what every verdict reader is given, is declared here, where the readers first need
//! it. The core builds it (`interpretRun`, or the dispatch), so it is never the host's value: its
//! output parser is the core's own stage, or the host's.

use serde::Deserialize;

use super::types::ParseResult;
use crate::format::issues::{PluginIssue, warning};
use crate::hooks::contracts::HookSource;
use crate::hooks::output::stage::OutputStage;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::registry::{Callback, Port};

/// The output stage's reading of a JSON value (`ParseOutput`): the core's own stage, the host's
/// stage object, or the host's function.
#[derive(Clone, Copy, Debug)]
pub enum Parser {
    /// The real stage (`createOutputStage`).
    Core,
    /// A stage the host gave, called as its method `parse`.
    Stage(Port),
    /// The host's function.
    Host(Callback),
}

/// What reading one run needs to know about the call it belongs to.
#[derive(Clone, Debug)]
pub struct Subject {
    /// The registered handler, as the host gave it.
    pub entry: JsValue,
    /// The event.
    pub event: String,
    /// The facts the event fired with (`source` of `ConfigChange`).
    pub facts: JsValue,
    /// `Bash` of a `PreToolUse:Bash` hook; absent on an event with no tool.
    pub tool_name: Option<String>,
    /// The output stage's reading of a JSON value (`ParseOutput`).
    pub parse: Parser,
    /// A Monitor call reached the hook through a `Bash` matcher ("allow and updatedInput have no
    /// effect").
    pub effects_ignored: bool,
    /// The limit the run was given in milliseconds, for the notice of a timeout; absent for none.
    pub timeout_ms: Option<f64>,
    /// Windows without Git Bash: a shell-form `PreToolUse` or `PermissionRequest` hook is a gate
    /// (hooks.md, "Failed hooks on Windows without Git Bash").
    pub git_bash_missing: bool,
    /// `TaskCompleted` fired because a teammate finished its turn, not because the `TaskUpdate`
    /// tool completed a task.
    pub teammate_stopped: bool,
}

impl Subject {
    /// The handler's id.
    #[must_use]
    pub fn entry_id(&self) -> String {
        self.entry
            .get("id")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    /// The handler's configuration (`entry.handler`).
    #[must_use]
    pub fn handler(&self) -> JsObject {
        self.entry
            .get("handler")
            .and_then(JsValue::as_object)
            .cloned()
            .unwrap_or_default()
    }

    /// Where the handler was declared.
    #[must_use]
    pub fn source(&self) -> Option<HookSource> {
        HookSource::deserialize(self.entry.get("source")?.clone()).ok()
    }

    /// `subject.parse(event, json)`: the output stage's reading of `json`.
    pub async fn parse(&self, event: &str, json: JsValue) -> ParseResult {
        let args = vec![JsValue::from(event), json];
        match self.parse {
            Parser::Host(parse) => parse.call::<ParseResult>(args).await,
            Parser::Stage(stage) => stage.method::<ParseResult>("parse", args).await,
            Parser::Core => {
                let json = args.into_iter().nth(1).unwrap_or_default();
                // An event the schemas do not know is no output to keep, as a failed check is.
                match OutputStage::parse(event, &json) {
                    Ok(parsed) => to_js_value(&parsed)
                        .ok()
                        .and_then(|value| ParseResult::deserialize(value).ok())
                        .unwrap_or_default(),
                    Err(failure) => ParseResult {
                        failure: Some(failure),
                        ..ParseResult::default()
                    },
                }
            }
        }
    }
}

/// hooks-guide.md, "Shell profile output before the JSON" (§7.4.1.1 item 5).
const PLAIN_TEXT_LINE: &str = "Hook output does not start with {, treating as plain text";

/// hooks-guide.md, "Fields at the wrong level" (§7.4.5.1 item 1, Judgment J7.30).
const UNRECOGNIZED_KEYS_LINE: &str = "Hook JSON output had unrecognized keys";

fn path_of(subject: &Subject) -> String {
    format!("hooks.{}.{}", subject.event, subject.entry_id())
}

/// The line for stdout read as plain text because it does not start with `{`; none for empty
/// stdout.
#[must_use]
pub fn plain_text_note(subject: &Subject, text: &str) -> Vec<PluginIssue> {
    if text.is_empty() || text.starts_with('{') {
        return vec![];
    }
    vec![warning(&path_of(subject), PLAIN_TEXT_LINE)]
}

/// The line for the keys a valid output had that the schema does not name, the keys after it.
#[must_use]
pub fn unrecognized_keys_note(subject: &Subject, keys: Option<&[String]>) -> Vec<PluginIssue> {
    match keys {
        Some(keys) if !keys.is_empty() => vec![warning(
            &path_of(subject),
            &format!("{UNRECOGNIZED_KEYS_LINE}: {}", keys.join(", ")),
        )],
        _ => vec![],
    }
}

/// `verdict` with `notes` added to its issues; every other member goes back as given.
#[must_use]
pub fn with_notes(mut verdict: JsObject, notes: &[PluginIssue]) -> JsObject {
    if notes.is_empty() {
        return verdict;
    }
    let mut issues = verdict
        .get("issues")
        .and_then(JsValue::as_array)
        .cloned()
        .unwrap_or_default();
    issues.extend(
        notes
            .iter()
            .map(|note| to_js_value(note).unwrap_or_default()),
    );
    verdict.insert("issues".to_owned(), JsValue::Array(issues));
    verdict
}
