//! Port of `src/rules/bash/wildcard.ts`, switched.
//!
//! The Bash rule pattern: a `*` stands for any text.

use serde::Deserialize;

use crate::js::slice_code_units;
use crate::rules::tool_pattern::wildcard_reg_exp;

/// `ls:*` is `ls *`: "an equivalent way to write a trailing wildcard", recognized only at the end.
#[must_use]
pub fn normalize_pattern(pattern: &str) -> String {
    pattern
        .strip_suffix(":*")
        .map_or_else(|| pattern.to_owned(), |without| format!("{without} *"))
}

/// Whether a Bash pattern holds a `*`, `:*` suffix included: an exact rule holds none.
#[must_use]
pub fn has_wildcard(pattern: &str) -> bool {
    pattern.contains('*')
}

/// How a pattern is matched.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternOptions {
    /// PowerShell matches without regard to case.
    #[serde(default)]
    pub ignore_case: Option<bool>,
}

/// Whether a command matches a Bash rule pattern (permissions.md, "Wildcard patterns"):
/// - a `*` stands for any text, spaces included, and everything else is literal;
/// - a `*` at the end with a space before it, when it is the only wildcard, also matches the bare
///   command: `ls *` matches `ls`, `* --help *` does not match `npm --help`;
/// - the space before a trailing `*` is part of the rule: `ls *` does not match `lsof`.
#[must_use]
pub fn match_bash_pattern(pattern: &str, subcommand: &str, options: PatternOptions) -> bool {
    let written = normalize_pattern(pattern);
    let ignore_case = options.ignore_case == Some(true);
    let flags = if ignore_case { "i" } else { "" };
    if wildcard_reg_exp(&written, Some(flags)).is_ok_and(|mut compiled| compiled.test(subcommand)) {
        return true;
    }
    let only_trailing = written.ends_with(" *") && written.matches('*').count() == 1;
    if !only_trailing {
        return false;
    }
    let bare = slice_code_units(&written, 0, Some(crate::js::code_unit_length(&written) - 2));
    if ignore_case {
        bare.to_lowercase() == subcommand.to_lowercase()
    } else {
        bare == subcommand
    }
}

crate::registry::exports! {
    "rules/bash/wildcard" {
        "matchBashPattern" => ok |pattern: String, subcommand: String, options: Option<PatternOptions>| match_bash_pattern(&pattern, &subcommand, options.unwrap_or_default());
    }
}
