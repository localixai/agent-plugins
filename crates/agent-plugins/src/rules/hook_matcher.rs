//! Port of `src/rules/hook-matcher.ts`, switched.
//!
//! How a hook's `matcher` is evaluated (hooks.md, "Matcher patterns"). The characters a matcher
//! holds decide how it reads:
//!
//! - `"*"`, `""` or omitted: matches every occurrence of the event;
//! - only exact-match characters: an exact string, or a list of exact strings;
//! - any other character: a JavaScript regular expression, tested unanchored
//!   (`RegExp.prototype.test`, so it succeeds on a match anywhere in the value).
//!
//! `FileChanged` and `StopFailure` use the narrower `narrow` set, so a hyphen, space or comma sends
//! their matcher to the regular-expression path and only `|` separates alternatives. Every other
//! event with matcher support uses `standard`. Which value an event is matched against, and that an
//! event without matcher support ignores the matcher, is the caller's (hooks/select).
//!
//! The core reads a matcher again on each call.

use serde::{Deserialize, Serialize};

use crate::js::regexp::RegExp;
use crate::js::{JsValue, string_of, trim};

/// Which grammar a matcher is read with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MatcherMode {
    /// `standard`.
    Standard,
    /// `narrow`.
    Narrow,
}

struct Grammar {
    /// A matcher made only of these characters is an exact string or list.
    exact: &'static str,
    /// What separates the alternatives of such a list.
    separator: &'static str,
}

const GRAMMARS: [(MatcherMode, Grammar); 2] = [
    (
        MatcherMode::Standard,
        Grammar {
            exact: r"^[A-Za-z0-9_\- ,|]+$",
            separator: "[|,]",
        },
    ),
    (
        MatcherMode::Narrow,
        Grammar {
            exact: r"^[A-Za-z0-9_|]+$",
            separator: r"\|",
        },
    ),
];

const MATCH_ALL: [&str; 2] = ["", "*"];

/// A regular expression of the module's own, which compiles.
fn literal(source: &str) -> Option<RegExp> {
    RegExp::new(source, "").ok()
}

/// The value a list alternative is compared with: surrounding whitespace is optional.
fn alternatives(matcher: &str, separator: &str) -> Vec<String> {
    literal(separator)
        .map(|separator| separator.split(matcher))
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .map(|alternative| trim(&alternative).to_owned())
        // Judgment (spec/judgments.md): an empty alternative such as the tail of `Edit|` names nothing.
        .filter(|alternative| !alternative.is_empty())
        .collect()
}

/// JavaScript's `RegExp.prototype.test`; a pattern that does not compile matches nothing (judgment).
fn pattern_selector(pattern: &str, value: &str) -> bool {
    RegExp::new(pattern, "").is_ok_and(|mut compiled| compiled.test(value))
}

/// How a matcher selects: the list it names, or its pattern compiled.
fn selector_of(grammar: &Grammar, matcher: &str, value: &str) -> bool {
    if !literal(grammar.exact).is_some_and(|mut exact| exact.test(matcher)) {
        return pattern_selector(matcher, value);
    }
    let names = alternatives(matcher, grammar.separator);
    names.iter().any(|name| name == value)
}

/// Whether `matcher` selects `value`. A match-all matcher selects even an event that has no value
/// to filter on; any other matcher needs a value (judgment: it matches nothing without one).
#[must_use]
pub fn matches_matcher(matcher: Option<&str>, value: Option<&str>, mode: MatcherMode) -> bool {
    let Some(matcher) = matcher.filter(|matcher| !MATCH_ALL.contains(matcher)) else {
        return true;
    };
    let Some(value) = value else {
        return false;
    };
    GRAMMARS
        .iter()
        .find(|(each, _)| *each == mode)
        .is_some_and(|(_, grammar)| selector_of(grammar, matcher, value))
}

/// `matchesMatcher` as the TypeScript ran it on any value an input holds, a schema-failing one
/// included: no list holds a value that is not a string, and a pattern tests `String(value)`.
fn matches_any_value(matcher: Option<&str>, value: &JsValue, mode: MatcherMode) -> bool {
    match value {
        JsValue::Undefined => matches_matcher(matcher, None, mode),
        JsValue::String(text) => matches_matcher(matcher, Some(text), mode),
        other @ (JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::Array(_)
        | JsValue::Object(_)) => {
            let Some(matcher) = matcher.filter(|matcher| !MATCH_ALL.contains(matcher)) else {
                return true;
            };
            GRAMMARS
                .iter()
                .find(|(each, _)| *each == mode)
                .is_some_and(|(_, grammar)| {
                    !literal(grammar.exact).is_some_and(|mut exact| exact.test(matcher))
                        && pattern_selector(matcher, &string_of(other))
                })
        }
    }
}

crate::registry::exports! {
    "rules/hook-matcher" {
        "matchesMatcher" => ok |matcher: Option<String>, value: JsValue, mode: Option<MatcherMode>| matches_any_value(matcher.as_deref(), &value, mode.unwrap_or(MatcherMode::Standard));
    }
}

#[cfg(test)]
mod tests {
    use super::{JsValue, MatcherMode, matches_any_value, matches_matcher};

    #[test]
    fn reads_lists_and_patterns() {
        let standard = MatcherMode::Standard;
        assert!(matches_matcher(
            Some(" Edit | Write "),
            Some("Write"),
            standard
        ));
        assert!(!matches_matcher(
            Some("Edit|Write"),
            Some("NotebookEdit"),
            standard
        ));
        assert!(matches_matcher(
            Some("mcp__.*__write.*"),
            Some("mcp__fs__write"),
            standard
        ));
        assert!(matches_matcher(
            Some("(?<=Note)book"),
            Some("Notebook"),
            standard
        ));
        assert!(!matches_matcher(Some("Edit("), Some("Edit("), standard));
        assert!(matches_matcher(
            Some("a,b"),
            Some("x a,b y"),
            MatcherMode::Narrow
        ));
        assert!(!matches_matcher(Some("Bash"), None, standard));
        // `matchesMatcher('5', 5)` and `matchesMatcher('^5$', 5)` in Bun: false, true.
        let five = JsValue::Number(5.0);
        assert!(!matches_any_value(Some("5"), &five, standard));
        assert!(matches_any_value(Some("^5$"), &five, standard));
        assert!(matches_any_value(Some("*"), &JsValue::Null, standard));
        assert!(matches_any_value(Some("^null$"), &JsValue::Null, standard));
        assert!(matches_matcher(None, None, standard));
    }
}
