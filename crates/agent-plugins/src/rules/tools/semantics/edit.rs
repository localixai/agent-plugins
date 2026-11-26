//! Port of `src/rules/tools/semantics/edit.ts`, switched.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};

/// The `old_string`, `new_string` and `replace_all` of an Edit call (hooks.md, "Edit").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditRequest {
    /// The text to find.
    pub old_string: String,
    /// The text to put in its place.
    pub new_string: String,
    /// Whether every occurrence is replaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_all: Option<bool>,
}

/// What the edit did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditResult {
    /// The file after the edit; the file as it was when `issues` is not empty.
    pub content: String,
    /// How many occurrences were replaced.
    pub replacements: usize,
    /// Why the edit was refused.
    pub issues: Vec<PluginIssue>,
}

const OLD_STRING: &str = "tool_input.old_string";

fn refused(content: &str, message: &str) -> EditResult {
    EditResult {
        content: content.to_owned(),
        replacements: 0,
        issues: vec![error(OLD_STRING, message)],
    }
}

/// Occurrences of `needle`, left to right and without overlap (spec/judgments.md: the docs say
/// "exactly once" and do not say how `aa` counts in `aaa`).
fn occurrences(content: &str, needle: &str) -> usize {
    content.matches(needle).count()
}

/// The Edit tool's replacement (tools-reference.md, "Edit tool behavior"): exact string replacement,
/// no regex and no fuzzy matching. `old_string` must be in the file as written (Match) and appear
/// exactly once unless `replace_all` is true (Uniqueness). `new_string` is inserted as plain text,
/// so `$&` stays `$&`. An empty `old_string` has nothing to find and is refused (spec/judgments.md),
/// as are the wordings of the refusals. Reading and writing the file, and the read-before-edit
/// check, are the host's.
#[must_use]
pub fn apply_edit(content: &str, request: &EditRequest) -> EditResult {
    let EditRequest {
        old_string: find,
        new_string: replacement,
        replace_all: all,
    } = request;
    let all = *all == Some(true);
    if find.is_empty() {
        return refused(content, "old_string is empty: there is no text to find");
    }
    let found = occurrences(content, find);
    if found == 0 {
        return refused(
            content,
            "old_string was not found in the file: it must appear exactly as written, including whitespace and indentation",
        );
    }
    if found > 1 && !all {
        return refused(
            content,
            &format!(
                "old_string appears {found} times in the file: add surrounding context to make it unique, or set replace_all to true"
            ),
        );
    }
    let edited = if all {
        content.replace(find.as_str(), replacement)
    } else {
        content.replacen(find.as_str(), replacement, 1)
    };
    EditResult {
        content: edited,
        replacements: if all { found } else { 1 },
        issues: Vec::new(),
    }
}

crate::registry::exports! {
    "rules/tools/semantics/edit" {
        "applyEdit" => ok |content: String, request: EditRequest| apply_edit(&content, &request);
    }
}
