//! Port of `src/marketplace/semver.ts`, switched.
//!
//! The range side of `semver` (node-semver; `format/versions.ts` orders releases with it), the
//! range grammar plugins/dependencies.md names for a dependency's `version` ("a semantic-version
//! range such as `~2.1.0`, `^2.0`, `>=1.4`, or `=2.1.0`", linking github.com/npm/node-semver#ranges).
//! Every rule below is node-semver's own, with its default options: a range never matches a
//! pre-release version unless it opts in with a pre-release suffix on a comparator ("`^2.0.0-0`").
//! node-semver itself is `crate::semver`, ported by hand.

use crate::js::{is_whitespace, trim};
use crate::semver::{Range, intersects, max_satisfying, parse, rcompare, satisfies, valid_range};

/// Whether `range` is a valid node-semver range.
#[must_use]
pub fn is_valid_range(range: &str) -> bool {
    valid_range(range).is_some()
}

/// A strict version (`2.1.0`, `2.0.0-beta.1`), as a git tag carries it: no `v`, `=` or blanks.
#[must_use]
pub fn strict_version(text: &str) -> Option<String> {
    let parsed = parse(text)?;
    // `/^[=v]|\s/i`
    let prefixed = text.starts_with(['=', 'v', 'V']);
    if parsed.raw != text || prefixed || text.chars().any(is_whitespace) {
        return None;
    }
    Some(parsed.version)
}

/// The version `text` names, spelled one way (`v2.1.100` is `2.1.100`), or `None` when it is no
/// version.
#[must_use]
pub fn normalized_version(text: &str) -> Option<String> {
    parse(text).map(|parsed| parsed.version)
}

/// Whether `version` satisfies `range`; a version that is not semver satisfies nothing.
#[must_use]
pub fn satisfies_range(version: &str, range: &str) -> bool {
    valid_range(range).is_some() && satisfies(version, range)
}

/// The highest of `versions` inside `range`, or `None`.
#[must_use]
pub fn highest_satisfying(versions: &[String], range: &str) -> Option<String> {
    valid_range(range)?;
    max_satisfying(versions, range)
}

/// The highest of `versions` by semver order, or `None` when there are none.
///
/// `versions.toSorted(rcompare)[0]`: a stable sort, so the first of equal versions; one version
/// is never compared. JavaScriptCore compares the second item with the first first, and meets the
/// items in their order, so of two or more the first that is no version is the one it throws for.
///
/// # Errors
///
/// `Invalid Version: <version>`, as `rcompare` throws it.
pub fn highest_version(versions: &[String]) -> Result<Option<String>, String> {
    if let [only] = versions {
        return Ok(Some(only.clone()));
    }
    let mut highest: Option<&String> = None;
    for version in versions {
        let first = highest.unwrap_or(version);
        if rcompare(version, first)? == -1 {
            highest = Some(version);
        } else {
            highest = Some(first);
        }
    }
    Ok(highest.cloned())
}

/// Beyond this many combinations a set of ranges is "too complex to intersect" (see
/// `ranges_overlap`).
const MAX_COMBINATIONS: usize = 64;

/// The `||` alternatives of a range, each a conjunction of comparators (`>=2.0.0 <3.0.0-0`): an
/// interval.
fn alternatives_of(range: &str) -> Result<Vec<String>, String> {
    let written = Range::new(range)?.range();
    Ok(written
        .split("||")
        .map(|one| trim(one).to_owned())
        .collect())
}

/// One alternative from each list, in every combination.
fn combinations(lists: &[Vec<String>]) -> Vec<Vec<&str>> {
    lists.iter().fold(vec![Vec::new()], |so, list| {
        so.iter()
            .flat_map(|chosen| {
                list.iter().map(move |one| {
                    let mut next = chosen.clone();
                    next.push(one.as_str());
                    next
                })
            })
            .collect()
    })
}

/// Whether intervals meet in a common point: in one dimension that is every pair (and each alone)
/// meeting.
fn meet(intervals: &[&str]) -> Result<bool, String> {
    for (index, a) in intervals.iter().enumerate() {
        if !intersects(a, a)? {
            return Ok(false);
        }
        for b in intervals.iter().skip(index + 1) {
            if !intersects(a, b)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Whether some version satisfies all of `ranges` (each valid), or `None` when there are too
/// many `||` alternatives to decide ("has version requirements too complex to intersect"). The
/// alternatives are distributed (`(a || b) && c` is `a && c || b && c`); the combination is empty
/// unless the chosen intervals meet pairwise, which for intervals means a common point (Helly).
///
/// # Errors
///
/// What node-semver throws for a range that is not valid.
pub fn ranges_overlap(ranges: &[String]) -> Result<Option<bool>, String> {
    let lists = ranges
        .iter()
        .map(String::as_str)
        .map(alternatives_of)
        .collect::<Result<Vec<_>, _>>()?;
    let count = lists
        .iter()
        .try_fold(1usize, |product, list| product.checked_mul(list.len()));
    if count.is_none_or(|count| count > MAX_COMBINATIONS) {
        return Ok(None);
    }
    for combination in combinations(&lists) {
        if meet(&combination)? {
            return Ok(Some(true));
        }
    }
    Ok(Some(false))
}

crate::registry::exports! {
    "marketplace/semver" {
        "isValidRange" => ok |range: String| is_valid_range(&range);
        "strictVersion" => ok |text: String| strict_version(&text);
        "normalizedVersion" => ok |text: String| normalized_version(&text);
        "satisfiesRange" => ok |version: String, range: String| satisfies_range(&version, &range);
        "highestSatisfying" => ok |versions: Vec<String>, range: String| highest_satisfying(&versions, &range);
        "highestVersion" => fallible |versions: Vec<String>| highest_version(&versions);
        "rangesOverlap" => fallible |ranges: Vec<String>| ranges_overlap(&ranges);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_of_equal_versions_is_the_highest() {
        let versions = ["1.0.0", "v1.0.0", "1.0.0+b"].map(str::to_owned);
        assert_eq!(highest_version(&versions), Ok(Some("1.0.0".to_owned())));
        let bad = ["nope", "also-not"].map(str::to_owned);
        assert_eq!(
            highest_version(&bad),
            Err("Invalid Version: nope".to_owned())
        );
        assert_eq!(
            ranges_overlap(&["^1.2".to_owned(), "1.4.x".to_owned()]),
            Ok(Some(true))
        );
    }
}
