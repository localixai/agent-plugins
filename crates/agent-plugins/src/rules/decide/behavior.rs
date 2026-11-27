//! Port of `src/rules/decide/behavior.ts`, switched.
//!
//! Whether a rule allows.

use crate::rules::restricts::restricts;
use crate::rules::types::PermissionRule;

/// Whether `rule` is an allow rule.
#[must_use]
pub const fn is_allow_rule(rule: &PermissionRule) -> bool {
    !restricts(rule.behavior)
}

crate::registry::exports! {
    "rules/decide/behavior" {
        "isAllowRule" => ok |rule: PermissionRule| is_allow_rule(&rule);
    }
}
