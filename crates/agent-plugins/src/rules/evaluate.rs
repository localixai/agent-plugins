//! The core's counterpart of `src/rules/evaluate.ts` at sha256:16efcfeef67e, which stays TypeScript: `evaluateRules` answers the caller's own rule object, which callers compare by identity, and both functions take a table of the host's matchers (docs/RUST.md, "What stays TypeScript").
//!
//! The core's callers (a hook's `if`) match with the kit's own matchers.

use serde::Serialize;

use super::matchers::specifier_matcher;
use super::param::matches_call_param;
use super::path::file_rules::routes_to_call;
use super::path::negation::is_carved_out;
use super::specifier_formats::rule_format_of;
use super::target::domain::domain_rule_covers_artifact_read;
use super::tool_pattern::tool_name_matches;
use super::types::{MatchContext, PermissionBehavior, PermissionRule, PreparedCall, Specifier};

/// Whether one rule covers one call. The tool name decides first (equal, or a glob matching it
/// whole). A bare rule then covers every use; a parameter rule looks at the call's input fields;
/// any other specifier goes to the matcher of the tool's rule format. The `Err` is what the
/// matcher throws.
pub fn match_rule(
    rule: &PermissionRule,
    call: &PreparedCall,
    ctx: &MatchContext,
) -> Result<bool, String> {
    if !tool_name_matches(rule, &call.call.tool_name) {
        // A path rule also covers the calls of the tools it stands for: an Edit rule Write, a Read rule Grep.
        if routes_to_call(rule, &call.call.tool_name) {
            return specifier_matcher(rule_format_of(&rule.value.tool_name))(rule, call, ctx);
        }
        // A `WebFetch(domain:...)` rule also covers an artifact read, which is a call of `Artifact`.
        return Ok(domain_rule_covers_artifact_read(rule, call));
    }
    match &rule.specifier {
        None => Ok(true),
        Some(specifier @ Specifier::Param { .. }) => Ok(matches_call_param(rule, specifier, call)),
        Some(Specifier::Domain { .. } | Specifier::Text { .. }) => {
            specifier_matcher(rule_format_of(&call.call.tool_name))(rule, call, ctx)
        }
    }
}

/// The order permissions.md ("Manage permissions") evaluates rules in: "deny, then ask, then
/// allow. The first match in that order determines the outcome, and rule specificity doesn't
/// change the order."
const PRECEDENCE: [PermissionBehavior; 3] = [
    PermissionBehavior::Deny,
    PermissionBehavior::Ask,
    PermissionBehavior::Allow,
];

/// What the rules alone say about a call: the first behaviour that matched, with the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleVerdict<'a> {
    /// The behaviour, `None` when no rule matched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionBehavior>,
    /// The rule that decided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<&'a PermissionRule>,
}

/// What the rules alone say about a call: the first deny rule that matches, else the first ask
/// rule, else the first allow rule, else nothing. A broad deny beats a narrower allow, and an ask
/// beats an allow, however specific. A `!` rule of the same source and list carves the paths it
/// matches out of the path rules before it, so such a rule is skipped for those paths.
pub fn evaluate_rules<'a>(
    rules: &'a [PermissionRule],
    call: &PreparedCall,
    ctx: &MatchContext,
) -> Result<RuleVerdict<'a>, String> {
    for behavior in PRECEDENCE {
        for (index, candidate) in rules.iter().enumerate() {
            if candidate.behavior == behavior
                && match_rule(candidate, call, ctx)?
                && !is_carved_out(rules, index, call, ctx)?
            {
                return Ok(RuleVerdict {
                    behavior: Some(behavior),
                    rule: Some(candidate),
                });
            }
        }
    }
    Ok(RuleVerdict {
        behavior: None,
        rule: None,
    })
}
