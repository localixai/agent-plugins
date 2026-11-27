//! Ported from `src/rules/decide/rules-stage.ts`; only the core uses it now, so it has no wrapper.

use super::behavior::is_allow_rule;
use super::context::DecideContext;
use super::facts::CallFacts;
use crate::js::{JsObject, JsValue};
use crate::rules::evaluate::{evaluate_rules, match_rule};
use crate::rules::types::{PermissionBehavior, PermissionRule, PreparedCall};

/// Every subcommand of a compound command must be allowed, "for example because the subcommand
/// matches an allow rule or is a read-only command" (permissions.md, "Compound commands"). The
/// rules may be different ones for different subcommands, so each subcommand is tried on its own
/// against every allow rule.
fn every_subcommand_allowed(
    call: &PreparedCall,
    rules: &[PermissionRule],
    facts: &CallFacts,
    ctx: &DecideContext,
) -> Result<bool, String> {
    let parts = &facts.parts;
    // A command of read-only parts alone needs no rule: the read-only gate settles it.
    if parts.subcommands.is_empty() || parts.read_only_parts.iter().all(|part| *part) {
        return Ok(false);
    }
    let allows: Vec<&PermissionRule> = rules.iter().filter(|rule| is_allow_rule(rule)).collect();
    for (index, subcommand) in parts.subcommands.iter().enumerate() {
        if parts.read_only_parts.get(index) == Some(&true) {
            continue;
        }
        let input = JsObject::from([("command".to_owned(), JsValue::from(subcommand.as_str()))]);
        let one = PreparedCall::of(&call.call.tool_name, JsValue::Object(input));
        let mut allowed = false;
        for rule in &allows {
            if match_rule(rule, &one, &ctx.match_context)? {
                allowed = true;
                break;
            }
        }
        if !allowed {
            return Ok(false);
        }
    }
    Ok(true)
}

/// What the rules in force say about a call: deny, then ask, then allow, the first match deciding
/// ("rule specificity doesn't change the order"). An allow also comes from a compound command whose
/// subcommands are each allowed by some allow rule or are read-only.
///
/// # Errors
///
/// What a rule's matcher throws.
pub fn rule_behavior(
    call: &PreparedCall,
    rules: &[PermissionRule],
    facts: &CallFacts,
    ctx: &DecideContext,
) -> Result<Option<PermissionBehavior>, String> {
    let verdict = evaluate_rules(rules, call, &ctx.match_context)?;
    if verdict.behavior.is_some() {
        return Ok(verdict.behavior);
    }
    let allowed = facts.shell && every_subcommand_allowed(call, rules, facts, ctx)?;
    Ok(allowed.then_some(PermissionBehavior::Allow))
}
