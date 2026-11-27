//! Differential checks of `src/ignore.rs` against npm's `ignore` itself, as
//! `scripts/rust/js-oracle/ignore.ts` recorded it in Bun: every pattern and list the oracle made,
//! tested against its paths with the case kept and ignored, and the longest pattern
//! JavaScriptCore compiles.

use agent_plugins::ignore::Ignore;

use crate::support::check_fallible;

const ORACLE: &str = "js/ignore";

#[test]
fn ignores_answers_as_npm_ignore() {
    check_fallible(
        ORACLE,
        "ignores",
        |(lines, ignorecase, path): (Vec<String>, bool, String)| {
            Ignore::new(ignorecase).add(&lines).ignores(&path)
        },
    );
}

/// `ignoresLong` is `ignores` of one line, `unit` repeated `count` times.
#[test]
fn a_pattern_too_large_for_javascript_core_throws_as_it_does() {
    check_fallible(
        ORACLE,
        "ignoresLong",
        |(unit, count, ignorecase, path): (String, usize, bool, String)| {
            Ignore::new(ignorecase)
                .add(&[unit.repeat(count)])
                .ignores(&path)
        },
    );
}
