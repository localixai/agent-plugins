//! Ported from `src/rules/decide/behavior.ts`; only the core uses it now, so it has no wrapper.
//!
//! Whether a rule allows.

use crate::rules::restricts::restricts;
use crate::rules::types::PermissionRule;

/// Whether `rule` is an allow rule.
#[must_use]
pub const fn is_allow_rule(rule: &PermissionRule) -> bool {
    !restricts(rule.behavior)
}
