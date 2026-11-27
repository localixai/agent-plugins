//! Port of `src/rules/rule-warnings.ts`, switched.

use serde::Serialize;

use crate::js::trim;
use crate::protocol::tool_names::REMOVED_TOOLS;
use crate::registry::Set;
use crate::rules::bash::text_pattern::text_pattern_of;
use crate::rules::restricts::restricts;
use crate::rules::types::{PermissionBehavior, PermissionRule, RuleSource};

/// The two startup warnings a rule list can draw, as values (spec 1.1.0 §13.2.1 item 3 and §13.5.2;
/// Judgment J13.33): where the row, the stderr line or the debug-log line goes is the presenting
/// client's, so the kit returns the parts and the caller writes the texts (errors.md, "Matches no
/// tool · check for typos" and "Is not matched by file permission checks"; `fileToolRuleBannerRow`,
/// `fileToolRuleWarning` and `matchesNoToolBannerRow` write them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RuleWarningKind {
    /// `matches no tool`.
    #[serde(rename = "matches no tool")]
    MatchesNoTool,
    /// `never matches`.
    #[serde(rename = "never matches")]
    NeverMatches,
}

/// `Edit` or `Read`, the rule kind file checks consult.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Use {
    /// `Edit`.
    Edit,
    /// `Read`.
    Read,
}

/// One warning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuleWarning {
    /// The rule as written, trimmed.
    pub rule: String,
    /// What the rule does.
    pub behavior: PermissionBehavior,
    /// Where it was written.
    pub source: RuleSource,
    /// Which warning.
    pub kind: RuleWarningKind,
    /// `never matches` only: the rule to write instead, `Edit(<path>)`, or `Read(<path>)` for `Glob`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    /// `never matches` only: `Edit` or `Read`, the rule kind file checks consult.
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub r#use: Option<Use>,
}

/// "Tool names containing `_` or `*` are exempt", and the names of removed tools (permissions.md).
fn exempt_name(name: &str, known_tools: &Set<String>) -> bool {
    name.contains('_')
        || name.contains('*')
        || known_tools.members.iter().any(|known| known == name)
        || REMOVED_TOOLS.contains(&name)
}

fn matches_no_tool(rule: &PermissionRule, known_tools: &Set<String>) -> Vec<RuleWarning> {
    // Only deny and ask rules are checked for a tool they cannot match (permissions.md, "Tool name wildcards").
    if !restricts(rule.behavior) || exempt_name(&rule.value.tool_name, known_tools) {
        return Vec::new();
    }
    vec![RuleWarning {
        rule: rule.text.clone(),
        behavior: rule.behavior,
        source: rule.source.clone(),
        kind: RuleWarningKind::MatchesNoTool,
        replacement: None,
        r#use: None,
    }]
}

/// The file tools whose path rules file checks never consult, and the rule kind to use instead
/// (errors.md, "Is not matched by file permission checks": `Write`, `NotebookEdit`, `MultiEdit` or
/// `Glob`; Grep is not named there, so it draws no warning).
const NEVER_MATCHED: [(&str, Use); 4] = [
    ("Write", Use::Edit),
    ("NotebookEdit", Use::Edit),
    ("MultiEdit", Use::Edit),
    ("Glob", Use::Read),
];

/// "Except in `--allowedTools`, where Claude Code accepts a `Glob` rule without warning" (errors.md).
const fn silent_source(source: &RuleSource) -> bool {
    match source {
        RuleSource::Managed
        | RuleSource::EmbedderManaged
        | RuleSource::Project
        | RuleSource::Local
        | RuleSource::User
        | RuleSource::FlagSettings { .. }
        | RuleSource::Session => false,
        RuleSource::Cli => true,
    }
}

/// The tools whose path rule `--allowedTools` accepts without warning.
const SILENT_IN_ALLOWED_TOOLS: [&str; 1] = ["Glob"];

fn silently_accepted(rule: &PermissionRule) -> bool {
    !restricts(rule.behavior)
        && silent_source(&rule.source)
        && SILENT_IN_ALLOWED_TOOLS.contains(&rule.value.tool_name.as_str())
}

fn never_matches(rule: &PermissionRule) -> Vec<RuleWarning> {
    let used = NEVER_MATCHED
        .iter()
        .find(|(name, _)| *name == rule.value.tool_name)
        .map(|(_, used)| *used);
    let path = text_pattern_of(rule);
    // A bare name or `Tool(*)` matches every use and draws no warning (errors.md, "What to do").
    let (Some(used), Some(path)) = (used, path) else {
        return Vec::new();
    };
    if trim(path) == "*" || silently_accepted(rule) {
        return Vec::new();
    }
    let name = match used {
        Use::Edit => "Edit",
        Use::Read => "Read",
    };
    vec![RuleWarning {
        rule: rule.text.clone(),
        behavior: rule.behavior,
        source: rule.source.clone(),
        kind: RuleWarningKind::NeverMatches,
        replacement: Some(format!("{name}({path})")),
        r#use: Some(used),
    }]
}

/// The startup warnings of each rule, in rule order, as values the caller formats and routes.
#[must_use]
pub fn rule_warnings(rules: &[PermissionRule], known_tools: &Set<String>) -> Vec<RuleWarning> {
    rules
        .iter()
        .flat_map(|rule| {
            matches_no_tool(rule, known_tools)
                .into_iter()
                .chain(never_matches(rule))
        })
        .collect()
}

crate::registry::exports! {
    "rules/rule-warnings" {
        "ruleWarnings" => ok |rules: Vec<PermissionRule>, known_tools: Set<String>| rule_warnings(&rules, &known_tools);
    }
}
