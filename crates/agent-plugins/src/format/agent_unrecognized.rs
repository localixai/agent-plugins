//! Port of `src/format/agent-unrecognized.ts`, switched.
//!
//! The top-level fields of an agent file that Claude Code does not recognize (sub-agents.md,
//! "Frontmatter reference"), each with the supported field it most likely means.

use super::agent_fields::AGENT_FIELD_NAMES;
use super::agent_record::UnrecognizedField;
use crate::js::JsObject;

/// sub-agents.md: "The debug log names up to five top-level fields".
const MAX_REPORTED: usize = 5;

/// A name this far from a supported one (in edits) still reads as a misspelling of it.
const CLOSE: usize = 2;

/// The number of single-code-unit edits between two words (Levenshtein).
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<u16> = a.encode_utf16().collect();
    let b: Vec<u16> = b.encode_utf16().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (row, left) in a.iter().enumerate() {
        let mut current = vec![row + 1];
        for (column, right) in b.iter().enumerate() {
            let cost = usize::from(left != right);
            let deleted = previous.get(column + 1).copied().unwrap_or(0) + 1;
            let inserted = current.get(column).copied().unwrap_or(0) + 1;
            let replaced = previous.get(column).copied().unwrap_or(0) + cost;
            current.push(deleted.min(inserted).min(replaced));
        }
        previous = current;
    }
    previous.last().copied().unwrap_or(0)
}

/// The supported field a misspelling most likely means: the same letters in another case
/// (`maxturns`) or within two edits (`maxTurn`, `disalowedTools`), the nearest first. Judgment: the
/// docs say only "the likely intended name when a supported field is close" (spec/judgments.md).
fn closest(field: &str) -> Option<&'static str> {
    let lower = field.to_lowercase();
    let mut scored: Vec<(&'static str, usize)> = AGENT_FIELD_NAMES
        .into_iter()
        .map(|name| (name, distance(&lower, &name.to_lowercase())))
        .filter(|(_, edits)| *edits <= CLOSE)
        .collect();
    scored.sort_by_key(|(_, edits)| *edits);
    scored.first().map(|(name, _)| *name)
}

/// The top-level fields of an agent file that Claude Code does not recognize (sub-agents.md,
/// "Frontmatter reference": it "ignores a field it doesn't recognize without reporting an error";
/// the debug log names up to five, with the likely intended name). In file order.
#[must_use]
pub fn unrecognized_fields(fields: &JsObject) -> Vec<UnrecognizedField> {
    fields
        .keys()
        .filter(|field| !AGENT_FIELD_NAMES.contains(&field.as_str()))
        .take(MAX_REPORTED)
        .map(|field| UnrecognizedField {
            field: field.clone(),
            suggestion: closest(field).map(str::to_owned),
        })
        .collect()
}

crate::registry::exports! {
    "format/agent-unrecognized" {
        "unrecognizedFields" => ok |fields: JsObject| unrecognized_fields(&fields);
    }
}
