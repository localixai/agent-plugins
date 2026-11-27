//! Port of `src/format/validate/unknown-fields.ts`, switched.
//!
//! What `claude plugin validate` says about a key it does not know (oracle 2.1.319). Claude Code
//! ignores such a key at load time; the validator tells the author why it is probably there:
//! a field of the marketplace entry, another tool's manifest, or a misspelling.

use serde::Deserialize;

use super::finding::{Finding, warning_at};
use crate::js::code_unit_length;

/// Fields of a marketplace entry that a `plugin.json` does not read.
const MARKETPLACE_ONLY: [&str; 5] = ["category", "tags", "strict", "id", "source"];

/// Other tools' manifests, by the label the validator gives them: where it is commonly seen, then
/// its fields.
const COMMONLY_SEEN: [(&str, &[&str]); 6] = [
    (
        "a VS Code/Cursor extension manifest",
        &[
            "publisher",
            "categories",
            "engines",
            "activationEvents",
            "contributes",
            "preview",
            "icon",
        ],
    ),
    (
        "an npm package.json",
        &[
            "scripts",
            "bin",
            "files",
            "private",
            "contributors",
            "main",
            "type",
        ],
    ),
    (
        "an MCPB/DXT manifest",
        &["dxt_version", "mcpb_version", "compatibility"],
    ),
    (
        "an MCP server manifest",
        &["tools", "prompts", "server", "resources"],
    ),
    (
        "manifests across many tools",
        &[
            "privacy_policies",
            "_comment",
            "changelog",
            "logo",
            "readme",
            "support",
            "privacy_policy",
        ],
    ),
    ("a JSON Schema document", &["$comment", "$id"]),
];

/// Where a key is commonly seen, when it is a field of another tool's manifest.
fn commonly_seen(key: &str) -> Option<&'static str> {
    COMMONLY_SEEN
        .iter()
        .rev()
        .find(|(_, keys)| keys.contains(&key))
        .map(|(label, _)| *label)
}

/// A spelling other tools use for a field Claude Code reads under another name.
fn cross_tool(key: &str) -> Option<&'static str> {
    (key == "user_config").then_some("userConfig")
}

/// A misspelling is a known key at most this far away (compared without case, `_`, `-` and `$`),
/// and never more than half its own length: `npm` is not taken for `name`, `note` is.
const MAX_DISTANCE: usize = 2;
const MIN_LENGTH: usize = 3;

fn squash(key: &str) -> Vec<char> {
    key.to_lowercase()
        .chars()
        .filter(|point| point.is_ascii_lowercase() || point.is_ascii_digit())
        .collect()
}

fn distance(a: &[char], b: &[char]) -> usize {
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (row, left) in a.iter().enumerate() {
        let mut current = vec![row + 1];
        for (column, right) in b.iter().enumerate() {
            let same = usize::from(left != right);
            let above = previous.get(column + 1).copied().unwrap_or(0) + 1;
            let before = current.get(column).copied().unwrap_or(0) + 1;
            let diagonal = previous.get(column).copied().unwrap_or(0) + same;
            current.push(above.min(before).min(diagonal));
        }
        previous = current;
    }
    previous.get(b.len()).copied().unwrap_or(usize::MAX)
}

/// The known key the unknown one most likely meant, if any is close enough.
fn closest<'a>(key: &str, known: &'a [String]) -> Option<&'a str> {
    if code_unit_length(key) < MIN_LENGTH {
        return None;
    }
    let wanted = squash(key);
    let reach = MAX_DISTANCE.min(wanted.len() / 2);
    let mut best: Option<(&str, usize)> = None;
    for candidate in known {
        let far = distance(&wanted, &squash(candidate));
        if far <= reach && best.is_none_or(|(_, nearest)| far < nearest) {
            best = Some((candidate, far));
        }
    }
    best.map(|(found, _)| found)
}

const IGNORED_AT_LOAD: &str = "Claude Code ignores it at load time.";

/// Where the key was found: only a manifest is told that a key belongs in the marketplace entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldSite {
    /// A plugin manifest.
    #[default]
    Manifest,
    /// Anywhere else.
    Other,
}

/// The sites, in the package's order.
pub const FIELD_SITES: [&str; 2] = ["manifest", "other"];

/// Whether a key found at `site` is told it belongs in the marketplace entry.
const fn tells_entry_fields(site: FieldSite) -> bool {
    match site {
        FieldSite::Manifest => true,
        FieldSite::Other => false,
    }
}

/// The warning for a key `known` does not list, at `path`.
#[must_use]
pub fn unknown_field_finding(key: &str, known: &[String], path: &str, site: FieldSite) -> Finding {
    if let Some(cross) = cross_tool(key) {
        return warning_at(
            path,
            &format!(
                "Field '{key}' is the cross-tool spelling of Claude Code's '{cross}'. Rename it to '{cross}' for Claude Code to read it (the option shapes differ slightly — re-run validate after renaming to check). As-is, Claude Code ignores it at load time."
            ),
        );
    }
    if tells_entry_fields(site) && MARKETPLACE_ONLY.contains(&key) {
        return warning_at(
            path,
            &format!(
                "Field '{key}' belongs in the marketplace entry (marketplace.json), not plugin.json. It's harmless here but unused — {IGNORED_AT_LOAD}"
            ),
        );
    }
    if let Some(seen) = commonly_seen(key) {
        return warning_at(
            path,
            &format!(
                "Unknown field '{key}' (commonly seen in {seen}). Claude Code ignores unrecognized fields at load time, so it's safe to keep."
            ),
        );
    }
    if let Some(meant) = closest(key, known) {
        return warning_at(
            path,
            &format!(
                "Unknown field '{key}' — did you mean '{meant}'? Claude Code ignores unrecognized fields at load time, so this field has no effect."
            ),
        );
    }
    warning_at(path, &format!("Unknown field '{key}'. {IGNORED_AT_LOAD}"))
}

crate::registry::exports! {
    "format/validate/unknown-fields" {
        "FIELD_SITES" => ok || FIELD_SITES;
        "unknownFieldFinding" => ok |key: String, known: Vec<String>, path: Option<String>, site: Option<FieldSite>| unknown_field_finding(&key, &known, path.as_deref().unwrap_or(&key), site.unwrap_or_default());
    }
}
