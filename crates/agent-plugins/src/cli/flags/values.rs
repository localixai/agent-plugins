//! Port of `src/cli/flags/values.ts`, switched.
//!
//! What the flag strategies share when they read a value: a JSON text, and a value that holds
//! several space-separated entries ("`--channels`: space-separated list", "`--plugin-url`:
//! space-separated URLs in a single quoted value", "`--mcp-config`: JSON files or strings
//! (space-separated)", cli-reference.md, "CLI flags").

use crate::format::read_json::{self, JsonRead};
use crate::js::is_whitespace;

/// `{ value } | { problem }`: what the one JSON reader answers.
pub type Parsed = JsonRead;

/// The parsed JSON, or the parser's own message: the kit's one JSON reader.
#[must_use]
pub fn parse_json(text: &str) -> Parsed {
    read_json::parse_json(text)
}

/// The entries of one value, split on whitespace; blanks are dropped.
#[must_use]
pub fn words_of(value: &str) -> Vec<String> {
    value
        .split(is_whitespace)
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

crate::registry::exports! {
    "cli/flags/values" {
        "parseJson" => ok |text: String| parse_json(&text);
    }
}
