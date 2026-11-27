//! Port of `src/rules/tools/semantics/glob-cap.ts`, switched.
//!
//! The Glob tool's result cap, its null-byte check and its `.gitignore` switch (tools-reference.md,
//! "Glob tool behavior").

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::js::{JsValue, trim};
use crate::ports::environment::Environment;
use crate::registry::Port;

/// tools-reference.md, "Glob tool behavior": "capped at 100 files".
pub const GLOB_RESULT_CAP: usize = 100;

/// One file a Glob found.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobMatch {
    /// The path.
    pub path: String,
    /// Modification time, in milliseconds.
    pub mtime_ms: f64,
}

/// A Glob result as Claude receives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CappedGlob {
    /// The paths kept.
    pub paths: Vec<String>,
    /// "If the cap is hit, Claude sees a truncation flag in the result."
    pub truncated: bool,
}

/// A Glob result as Claude receives it: sorted by modification time and capped at 100 files, with
/// a truncation flag when files were left out (tools-reference.md, "Glob tool behavior"). The docs
/// do not say which end of the order survives the cap: the newest files do, and files of one time
/// keep their order of arrival (spec/judgments.md). Walking the disk and reading times are the
/// host's.
#[must_use]
pub fn cap_glob_matches(matches: &[GlobMatch]) -> CappedGlob {
    let mut sorted: Vec<&GlobMatch> = matches.iter().collect();
    // `toSorted` with `right.mtimeMs - left.mtimeMs`: stable, and a NaN difference is a tie.
    sorted.sort_by(|left, right| {
        right
            .mtime_ms
            .partial_cmp(&left.mtime_ms)
            .unwrap_or(Ordering::Equal)
    });
    CappedGlob {
        paths: sorted
            .iter()
            .take(GLOB_RESULT_CAP)
            .map(|found| found.path.clone())
            .collect(),
        truncated: sorted.len() > GLOB_RESULT_CAP,
    }
}

const NULL_BYTE: char = '\u{0}';

/// "A `pattern` or `path` value that contains a null byte returns an error asking Claude to remove
/// it" (tools-reference.md, "Glob tool behavior"): one issue at `tool_input.<field>` per field that
/// holds one. The wording is a judgment (spec/judgments.md); a value that is not a string is left
/// to `parseToolInput`.
#[must_use]
pub fn validate_glob_input(input: &JsValue) -> Vec<PluginIssue> {
    if !input.is_object() {
        return Vec::new();
    }
    ["pattern", "path"]
        .into_iter()
        .filter(|field| {
            input
                .get(field)
                .and_then(JsValue::as_str)
                .is_some_and(|value| value.contains(NULL_BYTE))
        })
        .map(|field| {
            error(
                &format!("tool_input.{field}"),
                &format!("{field} contains a null byte: remove it and search again"),
            )
        })
        .collect()
}

/// "Glob doesn't respect `.gitignore` by default ... To make Glob respect `.gitignore`, set
/// `CLAUDE_CODE_GLOB_NO_IGNORE=false`" (tools-reference.md; env-vars.md: "Set to `false`"). Only
/// the text `false` does; any other value, or none, leaves gitignored files in
/// (spec/judgments.md).
pub async fn glob_respects_ignore(env: &dyn Environment) -> bool {
    env.get("CLAUDE_CODE_GLOB_NO_IGNORE")
        .await
        .is_some_and(|value| trim(&value) == "false")
}

crate::registry::exports! {
    "rules/tools/semantics/glob-cap" {
        "GLOB_RESULT_CAP" => ok || GLOB_RESULT_CAP;
        "capGlobMatches" => ok |matches: Vec<GlobMatch>| cap_glob_matches(&matches);
        "validateGlobInput" => ok |input: JsValue| validate_glob_input(&input);
        "globRespectsIgnore" => async ok |env: Port| glob_respects_ignore(&env).await;
    }
}
