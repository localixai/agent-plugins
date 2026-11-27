//! Port of `src/rules/bash/if-eval.ts`, switched.

use serde::Serialize;

use super::dialect::{BASH_DIALECT, POWERSHELL_DIALECT, ShellDialect};
use super::shell::command_of;
use super::split::scan_command;
use super::text_pattern::text_pattern_of;
use super::wildcard::normalize_pattern;
use crate::format::issues::PluginIssue;
use crate::js::{self, JsValue};
use crate::rules::evaluate::match_rule;
use crate::rules::parse::{RuleOrigin, parse_rule};
use crate::rules::specifier_formats::rule_format_of;
use crate::rules::tool_pattern::tool_name_matches;
use crate::rules::tools::rule_formats::RuleFormatId;
use crate::rules::types::{
    MatchContext, PermissionBehavior, PermissionRule, PreparedCall, RuleSource, ToolCall,
};

/// Whether a hook's `if` lets its handler run, with what the kit found wrong in the rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IfResult {
    /// Whether the handler runs.
    pub runs: bool,
    /// What the kit found wrong in the rule.
    pub issues: Vec<PluginIssue>,
}

/// The tools whose command can be too opaque to say what it runs.
const fn reads_commands(format: RuleFormatId) -> bool {
    match format {
        RuleFormatId::Command | RuleFormatId::PowershellCommand => true,
        RuleFormatId::ReadPath
        | RuleFormatId::EditPath
        | RuleFormatId::SkillName
        | RuleFormatId::SubagentType
        | RuleFormatId::Domain
        | RuleFormatId::Bare => false,
    }
}

/// The shell a command of each format is read in.
const fn dialects(format: RuleFormatId) -> Option<&'static ShellDialect> {
    match format {
        RuleFormatId::Command => Some(&BASH_DIALECT),
        RuleFormatId::PowershellCommand => Some(&POWERSHELL_DIALECT),
        RuleFormatId::ReadPath
        | RuleFormatId::EditPath
        | RuleFormatId::SkillName
        | RuleFormatId::SubagentType
        | RuleFormatId::Domain
        | RuleFormatId::Bare => None,
    }
}

/// Where an `if` issue is reported: the field of the hook handler, not a permissions list.
const IF_PATH: &str = "hooks.if";

/// hooks.md ("Match Monitor commands"): a Monitor call also answers to a rule that names Bash ("the
/// hook still gets the Monitor call's input"); a Bash call never answers to a Monitor rule. The
/// names a call answers to, by its own tool name.
const ANSWERS_TO: [(&str, &[&str]); 1] = [("Monitor", &["Monitor", "Bash"])];

/// "Patterns that specify more than the command name": what is left of the pattern after its
/// trailing wildcard, `git push` for `git push *`, has more than one word. `rm *` and `rm` name
/// only the command.
fn specifies_more_than_name(pattern: &str) -> bool {
    let normalized = normalize_pattern(pattern);
    let name = normalized.strip_suffix(" *").unwrap_or(&normalized);
    js::trim(name).chars().any(js::is_whitespace)
}

/// hooks.md ("How `if` patterns match Bash commands"): "When Claude Code can't determine which
/// commands the Bash input runs, it runs your hook regardless of the pattern", and a pattern that
/// says more than the command name runs on `$()`, backticks or `$VAR`.
fn cannot_tell(rule: &PermissionRule, call: &PreparedCall) -> bool {
    let format = rule_format_of(&call.call.tool_name);
    if !tool_name_matches(rule, &call.call.tool_name) || !reads_commands(format) {
        return false;
    }
    let Some(pattern) = text_pattern_of(rule) else {
        return false;
    };
    let dialect = dialects(format).unwrap_or(&BASH_DIALECT);
    let scan = command_of(&call.call.input).map(|command| scan_command(command, Some(dialect)));
    match scan {
        Some(scan) if scan.parseable => specifies_more_than_name(pattern) && scan.substitutes,
        _ => true,
    }
}

/// What `parseRule` says about the `if` text, reported at the handler's field.
fn if_issue(issue: PluginIssue) -> PluginIssue {
    PluginIssue {
        path: IF_PATH.to_owned(),
        message: format!(
            "{}. The `if` filter does not apply, so the hook runs for every call",
            issue.message
        ),
        ..issue
    }
}

/// Whether a hook handler with this `if` runs for this tool call (hooks.md, "Common fields"): the
/// rule is read as permission rule syntax and matches the way a deny or ask rule does, any
/// subcommand being enough, except that a single-directory path pattern keeps the depth of an allow
/// rule (`depthOf`). A rule the kit cannot read runs the handler and reports why: the `if`
/// filter is best effort (spec/judgments.md). Which events evaluate `if` is the hooks engine's.
/// The `Err` is what a matcher throws.
pub fn evaluate_if(
    if_rule: &str,
    tool_name: &str,
    tool_input: &JsValue,
    ctx: &MatchContext,
) -> Result<IfResult, String> {
    let at = RuleOrigin {
        behavior: PermissionBehavior::Deny,
        source: RuleSource::Cli,
    };
    let parsed = parse_rule(if_rule, &at);
    let issues: Vec<PluginIssue> = parsed.issues.into_iter().map(if_issue).collect();
    let Some(rule) = parsed.rule else {
        return Ok(IfResult { runs: true, issues });
    };
    let names: &[&str] = ANSWERS_TO
        .iter()
        .find(|(name, _)| *name == tool_name)
        .map_or(&[], |(_, names)| *names);
    let calls: Vec<PreparedCall> = if names.is_empty() {
        vec![prepared(tool_name, tool_input)]
    } else {
        names
            .iter()
            .map(|name| prepared(name, tool_input))
            .collect()
    };
    // hooks.md, "Common fields": a single-directory path pattern is not widened to every depth.
    let reading = MatchContext {
        depth_of: Some(PermissionBehavior::Allow),
        ..ctx.clone()
    };
    let mut runs = false;
    for call in &calls {
        if match_rule(&rule, call, &reading)? || cannot_tell(&rule, call) {
            runs = true;
            break;
        }
    }
    Ok(IfResult { runs, issues })
}

/// `{ toolName: name, input: toolInput }`.
fn prepared(tool_name: &str, input: &JsValue) -> PreparedCall {
    PreparedCall {
        call: ToolCall {
            tool_name: tool_name.to_owned(),
            input: input.clone(),
        },
        resolved_paths: None,
        skill_names: None,
        allowed_domains: None,
    }
}

crate::registry::exports! {
    "rules/bash/if-eval" {
        "evaluateIf" => fallible |if_rule: String, tool_name: String, tool_input: JsValue, ctx: MatchContext|
            evaluate_if(&if_rule, &tool_name, &tool_input, &ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_says_more_than_the_name_with_a_second_word() {
        assert!(specifies_more_than_name("git push *"));
        assert!(specifies_more_than_name("git push:*"));
        assert!(!specifies_more_than_name("rm *"));
        assert!(!specifies_more_than_name("rm"));
    }
}
