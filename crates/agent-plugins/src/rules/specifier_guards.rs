//! Ported from `src/rules/specifier-guards.ts`; only the core uses it now, so it has no wrapper.
//!
//! A guard for one form of specifier, so a matcher names what it reads and never repeats the
//! literal (K2).

use super::types::Specifier;

/// `Agent(Explore)`, `Skill(commit *)`: a name or pattern in the tool's own format; its pattern.
#[must_use]
pub fn is_text_specifier(specifier: Option<&Specifier>) -> Option<&str> {
    match specifier {
        Some(Specifier::Text { pattern }) => Some(pattern),
        _ => None,
    }
}

/// `WebFetch(domain:example.com)`: a domain pattern; its pattern.
#[must_use]
pub fn is_domain_specifier(specifier: Option<&Specifier>) -> Option<&str> {
    match specifier {
        Some(Specifier::Domain { pattern }) => Some(pattern),
        _ => None,
    }
}
