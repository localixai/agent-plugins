//! Differential checks of `src/semver.rs` against npm's `semver` itself, as
//! `scripts/rust/js-oracle/semver.ts` recorded it in Bun: every version and range the oracle
//! made, parsed, written, tested, compared and intersected.

use agent_plugins::marketplace::semver::highest_version;
use agent_plugins::semver::{
    Range, intersects, max_satisfying, parse, rcompare, satisfies, valid_range,
};
use serde_json::json;

use crate::support::{check, check_fallible};

const ORACLE: &str = "js/semver";

#[test]
fn parse_answers_as_npm_semver() {
    check(ORACLE, "parse", |(text,): (String,)| {
        parse(&text).map(|found| {
            json!({
                "version": found.version,
                "raw": found.raw,
                "prerelease": found.prerelease,
                "build": found.build,
            })
        })
    });
}

#[test]
fn a_range_is_written_as_npm_semver_writes_it() {
    check_fallible(ORACLE, "range", |(range,): (String,)| {
        Range::new(&range).map(|range| range.range())
    });
    check(ORACLE, "validRange", |(range,): (String,)| {
        valid_range(&range)
    });
}

#[test]
fn satisfies_answers_as_npm_semver() {
    check(ORACLE, "satisfies", |(version, range): (String, String)| {
        satisfies(&version, &range)
    });
    check(
        ORACLE,
        "maxSatisfying",
        |(versions, range): (Vec<String>, String)| max_satisfying(&versions, &range),
    );
}

#[test]
fn rcompare_answers_as_npm_semver() {
    check_fallible(ORACLE, "rcompare", |(a, b): (String, String)| {
        rcompare(&a, &b)
    });
}

#[test]
fn intersects_answers_as_npm_semver() {
    check_fallible(ORACLE, "intersects", |(a, b): (String, String)| {
        intersects(&a, &b)
    });
}

/// `marketplace/semver.ts`'s `highestVersion` is `versions.toSorted(rcompare)[0]`: the oracle
/// recorded that expression, which of the versions JavaScriptCore throws for included.
#[test]
fn the_highest_version_is_what_a_sort_by_rcompare_puts_first() {
    check_fallible(ORACLE, "highestVersion", |(versions,): (Vec<String>,)| {
        highest_version(&versions)
    });
}
