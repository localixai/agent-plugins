//! Port of `src/convert/notes.ts`, switched.
//!
//! The notes of a conversion and the conversion made of them. A `to` is a `PluginForm` of
//! `src/loading/core/form.ts`: held as its string until that module is in the core.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::js::json_stringify_indented;

/// `stopped`: nothing is written; `not-converted`: the target form does not get it; `reported`:
/// carried over, but it acts differently or the target warns about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionOutcome {
    /// `stopped`.
    #[serde(rename = "stopped")]
    Stopped,
    /// `not-converted`.
    #[serde(rename = "not-converted")]
    NotConverted,
    /// `reported`.
    #[serde(rename = "reported")]
    Reported,
}

/// One note of a conversion (Agent Plugins Extended §18.4.1 item 1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionNote {
    /// What became of the thing the note is about.
    pub outcome: ConversionOutcome,
    /// The file the note is about, relative to the plugin root (`mcp.json`, `commands/deploy.md`).
    pub path: String,
    /// Where in the file, dotted as the readers name it; absent for a whole file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// What the note says.
    pub message: String,
}

/// A file to write, relative to the plugin root, `/`-separated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertedFile {
    /// Relative to the plugin root.
    pub path: String,
    /// The file's text.
    pub text: String,
}

/// What a conversion gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversion {
    /// The form the files describe.
    pub to: String,
    /// A `stopped` note holds: `files` is empty.
    pub stopped: bool,
    /// The files to write.
    pub files: Vec<ConvertedFile>,
    /// What it could not carry over, or carried over with a difference.
    pub notes: Vec<ConversionNote>,
    /// The interpreter a written hook adapter needs on `PATH` (§18.4.4): `node`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
}

/// One note of a conversion (Agent Plugins Extended §18.4.1 item 1).
fn note(
    outcome: ConversionOutcome,
    path: String,
    field: Option<String>,
    message: String,
) -> ConversionNote {
    ConversionNote {
        outcome,
        path,
        field,
        message,
    }
}

/// A note that stops the conversion.
#[must_use]
pub fn stop(path: String, field: Option<String>, message: String) -> ConversionNote {
    note(ConversionOutcome::Stopped, path, field, message)
}

/// A note for what the target form does not get.
#[must_use]
pub fn not_converted(path: String, field: Option<String>, message: String) -> ConversionNote {
    note(ConversionOutcome::NotConverted, path, field, message)
}

/// A note for what is carried over with a difference.
#[must_use]
pub fn reported(path: String, field: Option<String>, message: String) -> ConversionNote {
    note(ConversionOutcome::Reported, path, field, message)
}

/// Which outcomes keep a conversion from writing anything.
const fn stops(outcome: ConversionOutcome) -> bool {
    matches!(outcome, ConversionOutcome::Stopped)
}

/// The conversion of the parts: no files when any note stops it.
#[must_use]
pub fn conversion(
    to: String,
    files: Vec<ConvertedFile>,
    notes: Vec<ConversionNote>,
    runtime: Option<String>,
) -> Conversion {
    let stopped = notes.iter().any(|found| stops(found.outcome));
    Conversion {
        to,
        stopped,
        files: if stopped { Vec::new() } else { files },
        notes,
        runtime: if stopped { None } else { runtime },
    }
}

/// A JSON file as the converter writes it: two-space indent and a final newline, so output is stable (§18.4.1 item 5).
#[must_use]
pub fn json_file(path: String, value: &JsValue) -> ConvertedFile {
    let text = json_stringify_indented(value, 2).unwrap_or_else(|| "undefined".to_owned());
    ConvertedFile {
        path,
        text: format!("{text}\n"),
    }
}

crate::registry::exports! {
    "convert/notes" {
        "stop" => ok |path: String, field: Option<String>, message: String| stop(path, field, message);
        "notConverted" => ok |path: String, field: Option<String>, message: String| not_converted(path, field, message);
        "reported" => ok |path: String, field: Option<String>, message: String| reported(path, field, message);
        "conversion" => ok |to: String, files: Vec<ConvertedFile>, notes: Vec<ConversionNote>, runtime: Option<String>| conversion(to, files, notes, runtime);
        "jsonFile" => ok |path: String, value: JsValue| json_file(path, &value);
    }
}
