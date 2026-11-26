//! Port of `src/rules/tools/semantics/read-window.ts`, switched.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::js::{number_to_string, slice_index};

/// The window a Read call asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReadRequest {
    /// The line to start at, counting from 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<f64>,
    /// How many lines to read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
}

/// One line with its number.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumberedLine {
    /// Its number, from 1.
    pub number: f64,
    /// Its text.
    pub text: String,
}

/// The window of a file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadSlice {
    /// The lines in the whole file, whatever the window.
    pub line_count: usize,
    /// What was wrong with the window asked for.
    pub issues: Vec<PluginIssue>,
    /// The lines of the window.
    pub lines: Vec<NumberedLine>,
    /// The notice that stands in for the lines: an empty file, or an offset past the last line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

/// A line ends at a newline or a carriage return and newline; a final newline starts no line.
fn lines_of(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = content.split('\n').collect();
    let last = lines.len().saturating_sub(1);
    for line in lines.iter_mut().take(last) {
        *line = line.strip_suffix('\r').unwrap_or(line);
    }
    if content.ends_with('\n') {
        lines.pop();
    }
    lines
}

/// `Number.isInteger(value) && value >= 1`.
fn is_whole_number(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0 && value >= 1.0
}

/// A window value that is not a whole number from 1 is an issue and counts as absent (spec/judgments.md).
fn window_value(value: Option<f64>, field: &str) -> (Option<f64>, Vec<PluginIssue>) {
    match value {
        None => (None, Vec::new()),
        Some(value) if is_whole_number(value) => (Some(value), Vec::new()),
        Some(_) => (
            None,
            vec![error(
                &format!("tool_input.{field}"),
                &format!("{field} must be a whole number of 1 or more"),
            )],
        ),
    }
}

fn plural(count: usize) -> String {
    format!("{count} line{}", if count == 1 { "" } else { "s" })
}

/// The window of a file the Read tool returns (tools-reference.md, "Read tool behavior"): the
/// contents "with line numbers", from the line `offset` ("line number to start reading from",
/// counted from 1: spec/judgments.md) for `limit` lines. An empty file and an `offset` past the last
/// line carry a notice instead of lines, the second giving the file's line count; the wording is a
/// judgment. The token and byte limits of large files are measured by the host.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "a line count is far below 2^53, as a JavaScript array's length is"
)]
pub fn read_window(content: &str, request: ReadRequest) -> ReadSlice {
    let (offset, offset_issues) = window_value(request.offset, "offset");
    let (limit, limit_issues) = window_value(request.limit, "limit");
    let issues: Vec<PluginIssue> = offset_issues.into_iter().chain(limit_issues).collect();
    let all = lines_of(content);
    let line_count = all.len();
    if all.is_empty() {
        return ReadSlice {
            lines: Vec::new(),
            line_count,
            notice: Some("The file exists but its contents are empty.".to_owned()),
            issues,
        };
    }
    let first = offset.unwrap_or(1.0);
    if first > line_count as f64 {
        return ReadSlice {
            lines: Vec::new(),
            line_count,
            notice: Some(format!(
                "The offset {} is past the last line: the file has {}.",
                number_to_string(first),
                plural(line_count)
            )),
            issues,
        };
    }
    let start = slice_index(first - 1.0, line_count);
    let end = limit.map_or(line_count, |limit| {
        slice_index(first - 1.0 + limit, line_count)
    });
    let taken = all.get(start..end.max(start)).unwrap_or_default();
    ReadSlice {
        lines: taken
            .iter()
            .enumerate()
            .map(|(index, text)| NumberedLine {
                number: first + index as f64,
                text: (*text).to_owned(),
            })
            .collect(),
        line_count,
        notice: None,
        issues,
    }
}

crate::registry::exports! {
    "rules/tools/semantics/read-window" {
        "readWindow" => ok |content: String, request: ReadRequest| read_window(&content, request);
    }
}
