//! Port of `src/rules/decide/auto-dropped.ts`, switched.
//!
//! permission-modes.md, "How the classifier evaluates actions": "On entering auto mode, broad allow
//! rules that grant arbitrary code execution are dropped": a blanket `Bash(*)` or `PowerShell(*)`,
//! wildcarded interpreters like `Bash(python*)`, package-manager run commands, `Agent` allow rules
//! and `Monitor` allow rules. "Narrow rules like `Bash(npm test)` stay in effect", and the dropped
//! rules come back when auto mode is left (they are only left out of this view of the rules).

use std::sync::LazyLock;

use regex::Regex;

use super::behavior::is_allow_rule;
use crate::js::{Pattern, regex_test};
use crate::rules::bash::text_pattern::text_pattern_of;
use crate::rules::types::PermissionRule;

/// A bare rule (`Bash(*)` reads as bare; `PowerShell(*)` is the pattern `*`, handled below) on one
/// of the tools whose every use runs code.
const BARE_TOOLS: [&str; 4] = ["Bash", "PowerShell", "Agent", "Monitor"];

/// The docs name `python`; the others are the same kind of interpreter, and the shell wrappers
/// `bash`, `sh` and `zsh` (with their `-c` forms) are the "shell-wrapper prefixes" of
/// settings-reference.md, `autoMode.classifyAllShell` (spec/judgments.md).
const INTERPRETERS: [&str; 8] = [
    "python", "python3", "node", "ruby", "perl", "bash", "sh", "zsh",
];

/// The docs say "package-manager run commands" without naming the managers (spec/judgments.md):
/// `/^(?:npm|yarn|pnpm|bun) run\b/`.
static PACKAGE_RUN: Pattern =
    LazyLock::new(|| Regex::new(r"^(?:npm|yarn|pnpm|bun) run(?-u:\b)").ok());

const SHELL_TOOLS: [&str; 2] = ["Bash", "PowerShell"];

/// `text.replace(/:?\*+$/, '')`.
fn without_trailing_stars(text: &str) -> &str {
    let stars = text.trim_end_matches('*');
    if stars.len() == text.len() {
        return text;
    }
    stars.strip_suffix(':').unwrap_or(stars)
}

fn wildcarded_interpreter(pattern: &str) -> bool {
    let program = without_trailing_stars(pattern.split(' ').next().unwrap_or_default());
    pattern.contains('*') && INTERPRETERS.contains(&program)
}

/// Whether `rule` is an allow rule auto mode drops. With `autoMode.classifyAllShell` "auto mode
/// suspends every shell allow rule" as well (settings-reference.md), `Bash(npm test)` included.
#[must_use]
pub fn is_dropped_in_auto(rule: &PermissionRule, classify_all_shell: bool) -> bool {
    if !is_allow_rule(rule) {
        return false;
    }
    let tool = rule.value.tool_name.as_str();
    if classify_all_shell && SHELL_TOOLS.contains(&tool) {
        return true;
    }
    if rule.specifier.is_none() {
        return BARE_TOOLS.contains(&tool);
    }
    SHELL_TOOLS.contains(&tool)
        && text_pattern_of(rule).is_some_and(|pattern| {
            pattern == "*" || wildcarded_interpreter(pattern) || regex_test(&PACKAGE_RUN, pattern)
        })
}

crate::registry::exports! {
    "rules/decide/auto-dropped" {
        "isDroppedInAuto" => ok |rule: PermissionRule, classify_all_shell: Option<bool>| is_dropped_in_auto(&rule, classify_all_shell.unwrap_or(false));
    }
}

#[cfg(test)]
mod tests {
    use super::without_trailing_stars;

    #[test]
    fn trailing_stars_go_with_one_colon_before_them() {
        assert_eq!(without_trailing_stars("python:**"), "python");
        assert_eq!(without_trailing_stars("python*"), "python");
        assert_eq!(without_trailing_stars("python:"), "python:");
        assert_eq!(without_trailing_stars("a::*"), "a:");
    }
}
