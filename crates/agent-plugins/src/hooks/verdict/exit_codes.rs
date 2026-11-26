//! Port of `src/hooks/verdict/exit-codes.ts`, switched.
//!
//! The table of "Exit code output" (hooks.md): what a hook's stdout and its exit code come to
//! together. The rows are the kinds of stdout, the columns the exit codes; each cell is what the
//! doc says, as data.

use indexmap::IndexMap;
use serde::Serialize;

/// The class of an exit code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExitClass {
    /// Exit 0.
    Zero,
    /// Exit 2.
    Two,
    /// Any other exit code.
    Other,
}

/// The class an exit code falls in.
#[must_use]
pub fn exit_class_of(exit_code: f64) -> ExitClass {
    if exit_code == 0.0 {
        ExitClass::Zero
    } else if exit_code == 2.0 {
        ExitClass::Two
    } else {
        ExitClass::Other
    }
}

/// What the run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultKind {
    /// "Success".
    Success,
    /// "Blocking error".
    Blocking,
    /// "Non-blocking error".
    NonBlockingError,
}

/// Whether the JSON fields count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fields {
    /// "The fields apply".
    Apply,
    /// "Claude Code still reads the fields, but they can't override the block".
    ReadNoOverride,
    /// "Claude Code ignores the exit code, and the fields alone decide".
    DecideAlone,
    /// They do not count.
    None,
}

/// What the message of the outcome is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MessageFrom {
    /// "Your stderr is the reason".
    Stderr,
    /// "The notice carries the parse or validation message".
    Validation,
    /// "The notice carries the first line of your stderr".
    FirstStderrLine,
    /// No message.
    None,
}

/// One cell of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Cell {
    /// What the run is.
    pub result: ResultKind,
    /// Whether the JSON fields count.
    pub fields: Fields,
    /// What the message is made of.
    pub message: MessageFrom,
    /// For a hook "whose failure blocks the action": the run counts as a failure. Doc: "Success. Claude
    /// Code ignores the exit code ... For hooks whose failure blocks the action, the run counts as a
    /// failure instead"; and "Plain-text stdout from a command hook isn't a failure" (exit 0).
    pub failure: bool,
}

const fn cell(
    result: ResultKind,
    fields: Fields,
    message: MessageFrom,
    failure: Option<bool>,
) -> Cell {
    Cell {
        result,
        fields,
        message,
        failure: match failure {
            Some(explicit) => explicit,
            None => matches!(result, ResultKind::NonBlockingError),
        },
    }
}

type Row = IndexMap<&'static str, Cell>;

fn row(valid: Cell, invalid: Cell, plain: Cell) -> Row {
    IndexMap::from([
        ("json-valid", valid),
        ("json-invalid", invalid),
        ("plain", plain),
    ])
}

/// The cell of each exit class and kind of stdout.
#[must_use]
pub fn exit_table() -> IndexMap<&'static str, Row> {
    IndexMap::from([
        (
            "zero",
            row(
                cell(ResultKind::Success, Fields::Apply, MessageFrom::None, None),
                cell(
                    ResultKind::NonBlockingError,
                    Fields::None,
                    MessageFrom::Validation,
                    None,
                ),
                cell(ResultKind::Success, Fields::None, MessageFrom::None, None),
            ),
        ),
        (
            "two",
            row(
                cell(
                    ResultKind::Blocking,
                    Fields::ReadNoOverride,
                    MessageFrom::Stderr,
                    None,
                ),
                cell(
                    ResultKind::Blocking,
                    Fields::None,
                    MessageFrom::Stderr,
                    None,
                ),
                cell(
                    ResultKind::Blocking,
                    Fields::None,
                    MessageFrom::Stderr,
                    None,
                ),
            ),
        ),
        (
            "other",
            row(
                cell(
                    ResultKind::Success,
                    Fields::DecideAlone,
                    MessageFrom::None,
                    Some(true),
                ),
                cell(
                    ResultKind::NonBlockingError,
                    Fields::None,
                    MessageFrom::Validation,
                    None,
                ),
                cell(
                    ResultKind::NonBlockingError,
                    Fields::None,
                    MessageFrom::FirstStderrLine,
                    None,
                ),
            ),
        ),
    ])
}

crate::registry::exports! {
    "hooks/verdict/exit-codes" {
        "exitClassOf" => ok |exit_code: f64| exit_class_of(exit_code);
        "EXIT_TABLE" => ok exit_table;
    }
}
