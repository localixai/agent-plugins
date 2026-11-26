//! Port of `src/format/validate/path-claims.ts`, switched.
//!
//! The paths a manifest names that the validator then looks for on disk (oracle 2.1.319). A path
//! with a `..` segment is refused outright; the rest are claims the caller checks against the files:
//! `Path not found`, or for `skills` a file where a directory is due.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use super::finding::{Finding, error_at};
use crate::js::{JsObject, JsValue, Pattern, property_order, regex_test};

/// A path a manifest claims to exist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathClaim {
    /// Where the oracle puts the report: `skills[0]`, `commands.a.source`, `mcpServers`.
    pub path: String,
    /// The path as written.
    pub value: String,
    /// The entry must be a directory holding `SKILL.md`.
    pub directory: bool,
    /// The loader skips a missing file here without reporting it (an MCP servers file).
    pub skipped_when_missing: bool,
}

/// The validator's message for a path with `..` (manifest-reference.md, "Containment and existence").
const TRAVERSAL: &str = "Path contains \"..\" which could be a path traversal attempt";

/// Keys whose single string is reported as the first entry of a list (`skills[0]`).
const LISTED: [&str; 4] = ["skills", "agents", "hooks", "workflows"];
/// Keys whose value is checked on disk.
const CHECKED: [&str; 11] = [
    "skills",
    "commands",
    "agents",
    "hooks",
    "mcpServers",
    "lspServers",
    "outputStyles",
    "workflows",
    "themes",
    "monitors",
    "types",
];

/// `^https?:\/\//i`, the ASCII letters folding only (RS6).
static REMOTE: Pattern = LazyLock::new(|| Regex::new(r"^(?i-u:https?)://").ok());

fn is_relative(value: &str) -> bool {
    !regex_test(&REMOTE, value)
}

fn entries_of(key: &str, value: Option<&JsValue>) -> Vec<(String, String)> {
    match value {
        Some(JsValue::String(text)) => {
            let path = if LISTED.contains(&key) {
                format!("{key}[0]")
            } else {
                key.to_owned()
            };
            vec![(path, text.clone())]
        }
        Some(JsValue::Array(items)) => items
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                entry
                    .as_str()
                    .map(|text| (format!("{key}[{index}]"), text.to_owned()))
            })
            .collect(),
        Some(JsValue::Object(commands)) if key == "commands" => property_order(commands)
            .into_iter()
            .filter_map(|(name, entry)| {
                let source = entry.get("source")?.as_str()?;
                Some((format!("commands.{name}.source"), source.to_owned()))
            })
            .collect(),
        Some(_) | None => Vec::new(),
    }
}

/// What the claims of a manifest are, and what is refused outright.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Claims {
    /// The paths to look for.
    pub claims: Vec<PathClaim>,
    /// The paths refused.
    pub findings: Vec<Finding>,
}

/// The paths of a manifest to look for on disk, and the ones refused for a `..`.
#[must_use]
pub fn claims_of(manifest: &JsObject) -> Claims {
    let mut all: Vec<PathClaim> = CHECKED
        .iter()
        .flat_map(|key| {
            entries_of(key, manifest.get(*key))
                .into_iter()
                .map(|(path, value)| PathClaim {
                    skipped_when_missing: *key == "mcpServers" && value.ends_with(".json"),
                    directory: *key == "skills",
                    path,
                    value,
                })
        })
        .collect();
    if let Some(JsValue::Object(experimental)) = manifest.get("experimental") {
        for key in ["themes", "monitors"] {
            all.extend(
                entries_of(&format!("experimental.{key}"), experimental.get(key))
                    .into_iter()
                    .map(|(path, value)| PathClaim {
                        path,
                        value,
                        directory: false,
                        skipped_when_missing: false,
                    }),
            );
        }
    }
    all.retain(|claim| is_relative(&claim.value));
    let (refused, claims): (Vec<PathClaim>, Vec<PathClaim>) = all
        .into_iter()
        .partition(|claim| claim.value.split('/').any(|segment| segment == ".."));
    Claims {
        claims,
        findings: refused
            .iter()
            .map(|claim| error_at(&claim.path, &format!("{TRAVERSAL}: {}", claim.value)))
            .collect(),
    }
}

crate::registry::exports! {
    "format/validate/path-claims" {
        "claimsOf" => ok |manifest: JsObject| claims_of(&manifest);
    }
}
