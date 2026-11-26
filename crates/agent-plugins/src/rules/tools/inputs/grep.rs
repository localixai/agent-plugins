//! Port of `src/rules/tools/inputs/grep.ts`, switched.

use serde::{Deserialize, Serialize};

/// hooks.md, "Grep": "`content`, `files_with_matches`, or `count`". The one list of the modes.
pub const GREP_OUTPUT_MODES: [GrepOutputMode; 3] = [
    GrepOutputMode::Content,
    GrepOutputMode::FilesWithMatches,
    GrepOutputMode::Count,
];

/// One of `GREP_OUTPUT_MODES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrepOutputMode {
    /// `content`.
    Content,
    /// `files_with_matches`.
    FilesWithMatches,
    /// `count`.
    Count,
}

crate::registry::exports! {
    "rules/tools/inputs/grep" {
        "GREP_OUTPUT_MODES" => ok || GREP_OUTPUT_MODES;
    }
}
