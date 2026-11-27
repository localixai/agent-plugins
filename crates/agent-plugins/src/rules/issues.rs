//! Port of `src/rules/issues.ts`, switched.
//!
//! The startup warnings of a rule list (errors.md, "Configuration warnings"), in the words of a
//! non-interactive text run; `ruleWarnings` gives the same warnings as values.

use super::bash::wildcard_warning::wildcard_before_subcommand;
use super::rule_warnings::{RuleWarning, RuleWarningKind, Use, rule_warnings};
use super::types::{PermissionBehavior, PermissionRule, RuleSource};
use crate::format::host_errors::{Behavior, FileToolRuleWarningInput, file_tool_rule_warning};
use crate::format::issues::{PluginIssue, warning};
use crate::format::settings_files::SETTINGS_FILE;
use crate::registry::Set;

const fn behavior_name(behavior: PermissionBehavior) -> &'static str {
    match behavior {
        PermissionBehavior::Allow => "allow",
        PermissionBehavior::Deny => "deny",
        PermissionBehavior::Ask => "ask",
    }
}

/// What the parentheses of the "is not matched by file permission checks" warning name: "a
/// settings file path, or the flag itself for `--allowed-tools` and `--disallowed-tools`", and
/// for managed settings "the full warning's parentheses read `managed policy settings`"
/// (errors.md).
fn source_of(warned: &RuleWarning) -> String {
    match &warned.source {
        RuleSource::Managed | RuleSource::EmbedderManaged => "managed policy settings".to_owned(),
        RuleSource::Project => SETTINGS_FILE.project.to_owned(),
        RuleSource::Local => SETTINGS_FILE.local.to_owned(),
        RuleSource::User => SETTINGS_FILE.user.to_owned(),
        RuleSource::FlagSettings { file } => file.clone(),
        RuleSource::Cli if warned.behavior == PermissionBehavior::Allow => {
            "--allowed-tools".to_owned()
        }
        RuleSource::Cli => "--disallowed-tools".to_owned(),
        RuleSource::Session => "session".to_owned(),
    }
}

const fn behavior_of(behavior: PermissionBehavior) -> Behavior {
    match behavior {
        PermissionBehavior::Allow => Behavior::Allow,
        PermissionBehavior::Deny => Behavior::Deny,
        PermissionBehavior::Ask => Behavior::Ask,
    }
}

/// The stderr form of each warning kind (errors.md, the non-interactive text run).
fn stderr_of(warned: &RuleWarning) -> String {
    match warned.kind {
        RuleWarningKind::MatchesNoTool => format!(
            "Permission {} rule \"{}\" matches no known tool — check for typos.",
            behavior_name(warned.behavior),
            warned.rule
        ),
        RuleWarningKind::NeverMatches => file_tool_rule_warning(&FileToolRuleWarningInput {
            behavior: behavior_of(warned.behavior),
            source: source_of(warned),
            rule: warned.rule.clone(),
            use_rule: match warned.r#use.unwrap_or(Use::Edit) {
                Use::Edit => "Edit".to_owned(),
                Use::Read => "Read".to_owned(),
            },
            replacement: warned.replacement.clone().unwrap_or_default(),
        }),
    }
}

fn issue_of(warned: &RuleWarning) -> PluginIssue {
    warning(
        &format!("permissions.{}", behavior_name(warned.behavior)),
        &stderr_of(warned),
    )
}

/// The startup warnings of a rule list (errors.md, "Configuration warnings"), in the words of a
/// non-interactive text run; `ruleWarnings` gives the same warnings as values.
#[must_use]
pub fn rule_issues(rules: &[PermissionRule], known_tools: &Set<String>) -> Vec<PluginIssue> {
    rules
        .iter()
        .flat_map(|rule| {
            let mut issues: Vec<PluginIssue> =
                rule_warnings(std::slice::from_ref(rule), known_tools)
                    .iter()
                    .map(issue_of)
                    .collect();
            issues.extend(wildcard_before_subcommand(rule));
            issues
        })
        .collect()
}

crate::registry::exports! {
    "rules/issues" {
        "ruleIssues" => ok |rules: Vec<PermissionRule>, known_tools: Set<String>| rule_issues(&rules, &known_tools);
    }
}
