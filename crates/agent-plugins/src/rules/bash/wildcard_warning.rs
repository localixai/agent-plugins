//! Port of `src/rules/bash/wildcard-warning.ts`, switched.
//!
//! The startup warning for a Bash allow rule with a `*` before the subcommand (errors.md, "Has a
//! wildcard before the rest of the command").

use super::text_pattern::text_pattern_of;
use super::wildcard::normalize_pattern;
use super::words::words_of;
use crate::format::issues::{PluginIssue, warning};
use crate::format::settings_files::SETTINGS_FILE;
use crate::rules::restricts::restricts;
use crate::rules::types::{PermissionRule, RuleSource};

fn is_option(word: &str) -> bool {
    word.starts_with('-')
}

fn has_star(word: &str) -> bool {
    word.contains('*')
}

/// Whether a `*` of the pattern stands where the subcommand, or an option before it, would be and
/// a literal word follows it: `git * main`, `git -C * status *`. errors.md ("Has a wildcard before
/// the rest of the command") describes "a `*` before a later word that determines which command it
/// is" and says to "only use `*` after the subcommand". The kit reads the subcommand as the first
/// word of the pattern after the program that is not an option, so a `*` after it (`git log *
/// main`, a documented rule) is not reported (spec/judgments.md).
fn has_wildcard_before_subcommand(pattern: &str) -> bool {
    let words: Vec<String> = words_of(&normalize_pattern(pattern), None)
        .into_iter()
        .map(|word| word.text)
        .collect();
    let Some((program, rest)) = words.split_first() else {
        return false;
    };
    if has_star(program) {
        return false;
    }
    let Some(star) = rest.iter().position(|word| has_star(word)) else {
        return false;
    };
    let (before, after) = rest.split_at(star);
    if !before.iter().all(|word| is_option(word)) {
        return false;
    }
    after.iter().skip(1).any(|word| !has_star(word))
}

/// The row shown below the startup banner in an interactive session (errors.md, "Has a wildcard
/// before the rest of the command"): `⚠ Allow rule Bash(git -C * status *) allows any option at
/// its * · use an exact value`.
#[must_use]
pub fn wildcard_banner_row(rule: &PermissionRule) -> String {
    format!(
        "⚠ Allow rule {} allows any option at its * · use an exact value",
        rule.text
    )
}

/// What follows `Permission allow rule` in parentheses: "a settings file path, or the
/// `--allowed-tools` flag itself". The file names are the documented locations of each source.
fn source_label(source: &RuleSource) -> &str {
    match source {
        RuleSource::Managed | RuleSource::EmbedderManaged => "managed settings",
        RuleSource::Project => SETTINGS_FILE.project,
        RuleSource::Local => SETTINGS_FILE.local,
        RuleSource::User => SETTINGS_FILE.user,
        RuleSource::FlagSettings { file } => file,
        RuleSource::Cli => "--allowed-tools",
        RuleSource::Session => "session",
    }
}

/// The part of the warning that is specific to git, whose options can run commands (errors.md).
const GIT_NOTE: &str = " For git, options such as -c and --exec-path can run arbitrary commands.";

/// errors.md names a `Bash` rule; Monitor and PowerShell are not named (spec/judgments.md).
const WARNED_TOOLS: [&str; 1] = ["Bash"];

/// The startup warning for a Bash allow rule with a `*` before the subcommand. Claude Code "keeps
/// the rule and changes nothing about how it matches", so this is only a warning.
#[must_use]
pub fn wildcard_before_subcommand(rule: &PermissionRule) -> Vec<PluginIssue> {
    // errors.md: the warning is about an allow rule (a deny or ask rule only restricts more).
    let text = text_pattern_of(rule).filter(|text| {
        !restricts(rule.behavior)
            && WARNED_TOOLS.contains(&rule.value.tool_name.as_str())
            && has_wildcard_before_subcommand(text)
    });
    let Some(text) = text else {
        return Vec::new();
    };
    let note = if text.starts_with("git ") {
        GIT_NOTE
    } else {
        ""
    };
    vec![warning(
        "permissions.allow",
        &format!(
            "Permission allow rule ({}): {} has a wildcard before the rest of the command, so it also matches any options inserted at that position and approves them without a prompt.{note} Replace that * with the exact value you mean, or only use * after the subcommand (for example Bash(git status *)).",
            source_label(&rule.source),
            rule.text
        ),
    )]
}

crate::registry::exports! {
    "rules/bash/wildcard-warning" {
        "wildcardBannerRow" => ok |rule: PermissionRule| wildcard_banner_row(&rule);
    }
}
