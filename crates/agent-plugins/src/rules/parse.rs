//! Port of `src/rules/parse.ts`, switched.

use serde::{Deserialize, Serialize};

use super::param::{is_primary_field, read_param};
use super::restricts::restricts;
use super::sources::is_settings_source;
use super::specifier_formats::{rule_format_of, specifier_syntax};
use super::tool_pattern::is_anchored_allow_glob;
use super::types::{
    PermissionBehavior, PermissionRule, PermissionRuleValue, RuleSource, Specifier,
};
use crate::format::issues::{PluginIssue, Severity};
use crate::format::permission_rule_shape::{
    MALFORMED_RULE, RuleParts, invalid_rule_message, split_rule,
};
use crate::js;

/// Where a rule was written and what it does: the two things the text cannot say.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleOrigin {
    /// What the rule does to a call it matches.
    pub behavior: PermissionBehavior,
    /// Where it was written.
    pub source: RuleSource,
}

/// A rule read, or why it was not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleParse {
    /// `None` when the rule is skipped or ignored; the issues say why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<PermissionRule>,
    /// Why it was skipped or ignored.
    pub issues: Vec<PluginIssue>,
}

/// What is wrong with a rule, before it becomes an issue at `permissions.<behavior>`.
struct Problem {
    severity: Severity,
    message: String,
}

fn invalid(raw: &str, reason: &str) -> Problem {
    Problem {
        severity: Severity::Error,
        message: invalid_rule_message(raw, reason),
    }
}

/// Rules a tool name rules out whatever follows it: an MCP rule with parentheses (in a settings
/// file only: the command line and the session read it as a parameter rule, permissions.md "Match
/// by input parameter"), a glob with a specifier.
fn check_tool_name(parts: &RuleParts, at: &RuleOrigin, raw: &str) -> Option<Problem> {
    let tool_name = &parts.tool_name;
    let has_specifier = parts.content.is_some();
    if has_specifier && tool_name.starts_with("mcp__") && is_settings_source(at.source.kind()) {
        return Some(invalid(
            raw,
            "an mcp__ rule in a settings file cannot have parentheses (to match a parameter of an MCP tool, pass a deny rule with --disallowedTools)",
        ));
    }
    if has_specifier && tool_name.contains('*') {
        return Some(invalid(raw, "a tool-name glob cannot take a specifier"));
    }
    if at.behavior == PermissionBehavior::Allow && !is_anchored_allow_glob(tool_name) {
        return Some(Problem {
            severity: Severity::Warning,
            message: format!(
                "Permission allow rule \"{raw}\" was skipped: an allow glob in the tool-name position must follow a literal mcp__<server>__ prefix, so it approves nothing"
            ),
        });
    }
    None
}

/// The pattern after a `domain:` prefix (`/^\s*domain\s*:\s*/`), trimmed.
fn domain_pattern(content: &str) -> Option<&str> {
    let rest = js::trim_start(content).strip_prefix("domain")?;
    let rest = js::trim_start(rest).strip_prefix(':')?;
    Some(js::trim(rest))
}

/// What the text between the parentheses means for `tool_name`.
fn read_specifier(
    parts: &RuleParts,
    at: &RuleOrigin,
    raw: &str,
) -> Result<Option<Specifier>, Problem> {
    let Some(content) = parts.content.as_deref() else {
        return Ok(None);
    };
    if js::trim(content).is_empty() {
        return Ok(None);
    }
    let syntax = specifier_syntax(rule_format_of(&parts.tool_name));
    if syntax.star_is_bare && js::trim(content) == "*" {
        return Ok(None);
    }
    if syntax.domain_prefix
        && let Some(pattern) = domain_pattern(content)
    {
        return Ok(Some(Specifier::Domain {
            pattern: pattern.to_owned(),
        }));
    }
    // Deny and ask rules read `name:value` as an input parameter; "allow rules continue to use each
    // tool's own specifier syntax" (permissions.md, "Match by input parameter").
    let param = if restricts(at.behavior) {
        read_param(content, &syntax)
    } else {
        None
    };
    let Some(Specifier::Param { name, value }) = param else {
        return Ok(Some(Specifier::Text {
            pattern: content.to_owned(),
        }));
    };
    if !is_primary_field(&parts.tool_name, &name) {
        return Ok(Some(Specifier::Param { name, value }));
    }
    let behavior = behavior_name(at.behavior);
    Err(Problem {
        severity: Severity::Warning,
        message: format!(
            "Permission {behavior} rule \"{raw}\" is ignored: {name} is the tool's primary content field and a parameter rule on it would be bypassable by a compound command. Use Bash(rm *), Read(./path) or WebFetch(domain:host) instead"
        ),
    })
}

/// A behavior as the TypeScript spells it.
const fn behavior_name(behavior: PermissionBehavior) -> &'static str {
    match behavior {
        PermissionBehavior::Allow => "allow",
        PermissionBehavior::Deny => "deny",
        PermissionBehavior::Ask => "ask",
    }
}

fn skip(at: &RuleOrigin, problem: Problem) -> RuleParse {
    RuleParse {
        rule: None,
        issues: vec![PluginIssue {
            severity: problem.severity,
            path: format!("permissions.{}", behavior_name(at.behavior)),
            message: problem.message,
            guidance: None,
        }],
    }
}

/// "In version 2.1.63, the Task tool was renamed to Agent. Existing `Task(...)` references in settings
/// and agent definitions still work as aliases." (sub-agents.md, "Restrict which subagents can be
/// spawned"): a rule names the tool as `Agent`, its `text` stays as written.
const RENAMED_TOOLS: [(&str, &str); 1] = [("Task", "Agent")];

fn renamed(parts: Option<RuleParts>) -> Option<RuleParts> {
    parts.map(|mut parts| {
        if let Some((_, to)) = RENAMED_TOOLS
            .iter()
            .find(|(from, _)| *from == parts.tool_name)
        {
            (*to).clone_into(&mut parts.tool_name);
        }
        parts
    })
}

/// Reads one permission rule (permissions.md, "Permission rule syntax"). A rule Claude Code skips or
/// ignores comes back as `rule: None` with the issue it reports; nothing throws.
#[must_use]
pub fn parse_rule(text: &str, at: &RuleOrigin) -> RuleParse {
    let raw = js::trim(text);
    let Some(parts) = renamed(split_rule(raw)) else {
        return skip(at, invalid(raw, MALFORMED_RULE));
    };
    if let Some(refused) = check_tool_name(&parts, at, raw) {
        return skip(at, refused);
    }
    let specifier = match read_specifier(&parts, at, raw) {
        Ok(specifier) => specifier,
        Err(problem) => return skip(at, problem),
    };
    let tool_glob = parts.tool_name.contains('*');
    let value = match (&specifier, parts.content) {
        (Some(_), Some(content)) => PermissionRuleValue {
            tool_name: parts.tool_name,
            rule_content: Some(content),
        },
        _ => PermissionRuleValue {
            tool_name: parts.tool_name,
            rule_content: None,
        },
    };
    RuleParse {
        rule: Some(PermissionRule {
            value,
            behavior: at.behavior,
            source: at.source.clone(),
            tool_glob,
            specifier,
            text: raw.to_owned(),
        }),
        issues: Vec::new(),
    }
}

/// A rule's text, as `parseRule` reads it back to the same value.
#[must_use]
pub fn format_rule(value: &PermissionRuleValue) -> String {
    match value.rule_content.as_deref() {
        None | Some("") => value.tool_name.clone(),
        Some(content) => format!("{}({content})", value.tool_name),
    }
}

crate::registry::exports! {
    "rules/parse" {
        "parseRule" => ok |text: String, at: RuleOrigin| parse_rule(&text, &at);
        "formatRule" => ok |value: PermissionRuleValue| format_rule(&value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str, behavior: PermissionBehavior) -> RuleParse {
        parse_rule(
            text,
            &RuleOrigin {
                behavior,
                source: RuleSource::User,
            },
        )
    }

    #[test]
    fn a_primary_field_is_ignored_and_a_domain_prefix_is_its_form() {
        let ignored = read("Bash(command:rm)", PermissionBehavior::Deny);
        assert_eq!(ignored.rule, None);
        assert_eq!(
            ignored.issues.first().map(|issue| issue.severity),
            Some(Severity::Warning)
        );
        let domain = read(
            "WebFetch( domain : example.com )",
            PermissionBehavior::Allow,
        );
        assert_eq!(
            domain.rule.and_then(|rule| rule.specifier),
            Some(Specifier::Domain {
                pattern: "example.com".to_owned()
            })
        );
        let task = read("Task(Explore)", PermissionBehavior::Allow);
        assert_eq!(
            task.rule.map(|rule| (rule.value.tool_name, rule.text)),
            Some(("Agent".to_owned(), "Task(Explore)".to_owned()))
        );
        let bare = read("Bash()", PermissionBehavior::Allow);
        assert_eq!(
            bare.rule.map(|rule| format_rule(&rule.value)),
            Some("Bash".to_owned())
        );
    }
}
