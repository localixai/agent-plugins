//! Port of `src/rules/tools/responses/structured-patch.ts`, switched.

use std::sync::LazyLock;

use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

use crate::js::{Pattern, number};

/// One hunk of an `Edit` or `Write` result's `structuredPatch` (the SDK's `FileEditOutput`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchHunk {
    /// The first line of the old side.
    pub old_start: f64,
    /// The lines of the old side.
    pub old_lines: f64,
    /// The first line of the new side.
    pub new_start: f64,
    /// The lines of the new side.
    pub new_lines: f64,
    /// The hunk's lines, each with its mark.
    pub lines: Vec<String>,
}

/// `/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/`, digits ASCII as JavaScript's `\d`.
static HUNK_HEADER: Pattern =
    LazyLock::new(|| Regex::new(r"^@@ -([0-9]+)(?:,([0-9]+))? \+([0-9]+)(?:,([0-9]+))? @@").ok());

/// A header's count is one when it is left out (`@@ -3 +3 @@`), as unified diffs write it.
fn count_of(text: Option<&str>) -> f64 {
    text.map_or(1.0, number)
}

/// What a hunk line uses up of the old and the new side: a marker line uses neither.
struct Uses {
    old: f64,
    new: f64,
}

/// `USES[mark]`, a context line for any other mark.
fn uses_of(mark: Option<char>) -> Uses {
    match mark {
        Some('-') => Uses { old: 1.0, new: 0.0 },
        Some('+') => Uses { old: 0.0, new: 1.0 },
        Some('\\') => Uses { old: 0.0, new: 0.0 },
        Some(_) | None => Uses { old: 1.0, new: 1.0 },
    }
}

/// The `\ No newline at end of file` lines from `from` on, added to `body`; where they end.
fn take_markers(lines: &[&str], from: usize, body: &mut Vec<String>) -> usize {
    let mut at = from;
    while let Some(line) = lines.get(at).filter(|line| line.starts_with('\\')) {
        body.push((*line).to_owned());
        at += 1;
    }
    at
}

/// A hunk's lines as the SDK keeps them: each starts with a space, `+` or `-`; a `\ No newline at
/// end of file` marker stays with the line it follows. The hunk ends where the header's counts say.
fn take_hunk(lines: &[&str], from: usize, header: &Captures<'_>) -> (PatchHunk, usize) {
    let group = |index: usize| header.get(index).map(|found| found.as_str());
    let (old_start, old_count) = (group(1).unwrap_or("0"), group(2));
    let (new_start, new_count) = (group(3).unwrap_or("0"), group(4));
    let mut old_left = count_of(old_count);
    let mut new_left = count_of(new_count);
    let mut body: Vec<String> = Vec::new();
    let mut at = from;
    while at < lines.len() && (old_left > 0.0 || new_left > 0.0) {
        let line = lines.get(at).copied().unwrap_or_default();
        let used = uses_of(line.chars().next());
        old_left -= used.old;
        new_left -= used.new;
        body.push(if line.is_empty() { " " } else { line }.to_owned());
        at += 1;
    }
    // A marker right after the last counted line belongs to it.
    at = take_markers(lines, at, &mut body);
    let hunk = PatchHunk {
        old_start: number(old_start),
        old_lines: count_of(old_count),
        new_start: number(new_start),
        new_lines: count_of(new_count),
        lines: body,
    };
    (hunk, at)
}

/// The hunks of a unified diff as the `structuredPatch` of an `Edit` or `Write` result
/// (agent-sdk/typescript.md, "Edit" and "Write" output: "Diff hunks"; empty when nothing changed).
/// The file headers (`---`, `+++`, `Index:`) are skipped, and so is any text that is no hunk.
/// Judgment: the docs name the field and its type and not how a host derives it from its own diff
/// (spec/judgments.md).
#[must_use]
pub fn structured_patch_of(diff: &str) -> Vec<PatchHunk> {
    let lines: Vec<&str> = diff.split('\n').collect();
    let mut hunks = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        let line = lines.get(at).copied().unwrap_or_default();
        let Some(header) = HUNK_HEADER
            .as_ref()
            .and_then(|pattern| pattern.captures(line))
        else {
            at += 1;
            continue;
        };
        let (hunk, next) = take_hunk(&lines, at + 1, &header);
        hunks.push(hunk);
        at = next;
    }
    hunks
}

crate::registry::exports! {
    "rules/tools/responses/structured-patch" {
        "structuredPatchOf" => ok |diff: String| structured_patch_of(&diff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_pattern_compiles() {
        assert!(HUNK_HEADER.is_some());
    }
}
