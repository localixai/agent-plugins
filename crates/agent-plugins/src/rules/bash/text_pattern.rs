//! Ported from `src/rules/bash/text-pattern.ts`; only the core uses it now, so it has no wrapper.

use crate::rules::types::{PermissionRule, Specifier};

/// The command, path or skill text of each specifier form: only a `text` specifier has one.
fn text_of(specifier: &Specifier) -> Option<&str> {
    match specifier {
        Specifier::Param { .. } | Specifier::Domain { .. } => None,
        Specifier::Text { pattern } => Some(pattern),
    }
}

/// The pattern a rule matches command text with, or `undefined` for a bare or non-text rule.
#[must_use]
pub fn text_pattern_of(rule: &PermissionRule) -> Option<&str> {
    rule.specifier.as_ref().and_then(text_of)
}
