//! Port of `src/hooks/verdict/debug-lines.ts`, switched.
//!
//! The debug-log lines the docs quote for how a hook's output was read, as warnings of the run: the
//! kit writes no log, the host does (judgment, spec/judgments.md; Agent Plugins Extended §7.4.12).
//!
//! `Subject` (`src/hooks/verdict/subject.ts`, types only) is declared here, where the TypeScript
//! first needs it: it holds the output parser, a function, so it crosses as the host's object and
//! its members are read as they are needed.

use serde::Deserialize;

use crate::format::issues::{PluginIssue, warning};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::registry::Port;

/// The part of a registered handler a subject's readers name.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SubjectEntry {
    /// The handler's id.
    #[serde(default)]
    pub id: String,
}

/// The members of a subject read as data.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SubjectData {
    /// The handler that ran.
    #[serde(default)]
    pub entry: SubjectEntry,
    /// The event.
    #[serde(default)]
    pub event: String,
}

/// What a run is read against (`Subject`): the host's object, or plain data.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Subject {
    /// The subject holds a function (the output parser).
    Host(Port),
    /// The subject is data.
    Data(SubjectData),
}

impl Subject {
    /// The handler's id and the event.
    pub async fn named(&self) -> SubjectData {
        match self {
            Self::Data(data) => data.clone(),
            Self::Host(port) => SubjectData {
                entry: port.member::<SubjectEntry>("entry").await,
                event: port.member::<String>("event").await,
            },
        }
    }
}

/// hooks-guide.md, "Shell profile output before the JSON" (§7.4.1.1 item 5).
const PLAIN_TEXT_LINE: &str = "Hook output does not start with {, treating as plain text";

/// hooks-guide.md, "Fields at the wrong level" (§7.4.5.1 item 1, Judgment J7.30).
const UNRECOGNIZED_KEYS_LINE: &str = "Hook JSON output had unrecognized keys";

async fn path_of(subject: &Subject) -> String {
    let named = subject.named().await;
    format!("hooks.{}.{}", named.event, named.entry.id)
}

/// The line for stdout read as plain text because it does not start with `{`; none for empty stdout.
pub async fn plain_text_note(subject: &Subject, text: &str) -> Vec<PluginIssue> {
    if text.is_empty() || text.starts_with('{') {
        return vec![];
    }
    vec![warning(&path_of(subject).await, PLAIN_TEXT_LINE)]
}

/// The line for the keys a valid output had that the schema does not name, the keys after it.
pub async fn unrecognized_keys_note(
    subject: &Subject,
    keys: Option<&[String]>,
) -> Vec<PluginIssue> {
    match keys {
        Some(keys) if !keys.is_empty() => vec![warning(
            &path_of(subject).await,
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

crate::registry::exports! {
    "hooks/verdict/debug-lines" {
        "plainTextNote" => async ok |subject: Subject, text: String|
            plain_text_note(&subject, &text).await;
        "unrecognizedKeysNote" => async ok |subject: Subject, keys: Option<Vec<String>>|
            unrecognized_keys_note(&subject, keys.as_deref()).await;
        "withNotes" => ok |verdict: JsObject, notes: Vec<PluginIssue>| with_notes(verdict, &notes);
    }
}
