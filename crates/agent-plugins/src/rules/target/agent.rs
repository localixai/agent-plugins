//! Ported from `src/rules/target/agent.ts`; only the core uses it now, so it has no wrapper.
//!
//! `Agent(AgentName)` (permissions.md, "Agent (subagents)"): the rule's text is the subagent's
//! name and is compared with the call's `subagent_type`. The comparison is exact and
//! case-sensitive. A fork is the call whose `subagent_type` is `fork` (the SDK's `AgentInput`
//! names it so), which `Agent(fork)` stops (sub-agents.md, "Turn fork mode off"). A call that
//! leaves `subagent_type` out is `general-purpose` for a deny or ask rule and no name for an allow
//! rule (spec/judgments.md).

use super::input::input_string;
use crate::rules::specifier_guards::is_text_specifier;
use crate::rules::types::{PermissionBehavior, PermissionRule, PreparedCall};

/// The subagent a call without `subagent_type` falls back to (sub-agents.md, "Built-in
/// subagents").
const FALLBACK_SUBAGENT: &str = "general-purpose";

/// What an omitted `subagent_type` counts as, per behaviour, on the safe side: a deny or ask rule
/// for `general-purpose` still covers the call, which runs as that subagent; an allow rule
/// approves nothing it cannot see named. A session with no `general-purpose` fails the call, so
/// the cover costs nothing there.
const fn name_when_omitted(behavior: PermissionBehavior) -> Option<&'static str> {
    match behavior {
        PermissionBehavior::Allow => None,
        PermissionBehavior::Deny | PermissionBehavior::Ask => Some(FALLBACK_SUBAGENT),
    }
}

/// The matcher of the `subagent-type` rule format.
#[must_use]
pub fn match_agent_rule(rule: &PermissionRule, call: &PreparedCall) -> bool {
    let Some(pattern) = is_text_specifier(rule.specifier.as_ref()) else {
        return false;
    };
    input_string(&call.call.input, "subagent_type")
        .or_else(|| name_when_omitted(rule.behavior))
        .is_some_and(|name| name == pattern)
}
