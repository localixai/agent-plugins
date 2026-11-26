//! A range (`classes/range.js`): `||` alternatives, each a set of comparators every version in it
//! satisfies. The package keeps a cache of parsed sets; a set is a pure function of its text, so
//! the core reads it again instead.

use indexmap::IndexMap;

use crate::js::{is_whitespace, trim};

use super::comparator::Comparator;
use super::re::{BUILD_STRIP, CARET_TRIM, HYPHEN_RANGE, TILDE_TRIM, replace, replace_with};
use super::replace::{hyphen_replace, parse_comparator, replace_gte0};
use super::trim::trim_comparator_whitespace;
use super::version::SemVer;

/// A parsed range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Range {
    /// The text with its blanks trimmed and collapsed to one space.
    pub raw: String,
    /// The alternatives, each a set of comparators.
    pub set: Vec<Vec<Comparator>>,
}

/// `text.split(/\s+/)`: the parts between runs of JavaScript's whitespace, an empty part before a
/// leading run and after a trailing one, and `[""]` for an empty text.
pub(super) fn split_blanks(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_blank = false;
    for (at, character) in text.char_indices() {
        if is_whitespace(character) {
            if !in_blank {
                parts.push(text.get(start..at).unwrap_or_default());
                in_blank = true;
            }
        } else if in_blank {
            start = at;
            in_blank = false;
        }
    }
    parts.push(if in_blank {
        ""
    } else {
        text.get(start..).unwrap_or_default()
    });
    parts
}

/// `text.replace(/\s+/g, ' ')`.
fn collapse_blanks(text: &str) -> String {
    split_blanks(text).join(" ")
}

fn is_null_set(comparator: &Comparator) -> bool {
    comparator.value == "<0.0.0-0"
}

fn is_any(comparator: &Comparator) -> bool {
    comparator.value.is_empty()
}

/// `parseRange(range)`: one alternative read into its comparators.
fn parse_range(range: &str) -> Result<Vec<Comparator>, String> {
    // Strip build metadata so it can't bleed into the version.
    let range = replace(&BUILD_STRIP, range, "");
    // `1.2.3 - 1.2.4` => `>=1.2.3 <=1.2.4`
    let range = replace_with(&HYPHEN_RANGE, &range, hyphen_replace);
    // `> 1.2.3 < 1.2.5` => `>1.2.3 <1.2.5`
    let range = trim_comparator_whitespace(&range);
    // `~ 1.2.3` => `~1.2.3`, `^ 1.2.3` => `^1.2.3`
    let range = replace(&TILDE_TRIM, &range, "$1~");
    let range = replace(&CARET_TRIM, &range, "$1^");
    let parsed: Vec<String> = range.split(' ').map(parse_comparator).collect();
    let joined = parsed.join(" ");
    let comparators = split_blanks(&joined)
        .into_iter()
        // `>=0.0.0` is equivalent to `*`.
        .map(replace_gte0)
        .map(|comp| Comparator::new(&comp))
        .collect::<Result<Vec<_>, _>>()?;
    // A null set is the whole set; more than one comparator drops `*`; none is kept twice.
    let mut by_value: IndexMap<String, Comparator> = IndexMap::new();
    for comparator in comparators {
        if is_null_set(&comparator) {
            return Ok(vec![comparator]);
        }
        by_value.insert(comparator.value.clone(), comparator);
    }
    if by_value.len() > 1 {
        by_value.shift_remove("");
    }
    Ok(by_value.into_values().collect())
}

/// `isSatisfiable(comparators)`: whether each comparator meets every one before it.
fn is_satisfiable(comparators: &[Comparator]) -> Result<bool, String> {
    let mut remaining = comparators.to_vec();
    let mut test = remaining.pop();
    while let Some(comparator) = test.take().filter(|_| !remaining.is_empty()) {
        for other in &remaining {
            if !comparator.intersects(other)? {
                return Ok(false);
            }
        }
        test = remaining.pop();
    }
    Ok(true)
}

/// `testSet(set, version)`: every comparator holds, and a pre-release is in only where a
/// comparator of the set names a pre-release of the same `major.minor.patch`.
fn test_set(set: &[Comparator], version: &SemVer) -> bool {
    if !set.iter().all(|comparator| comparator.test(version)) {
        return false;
    }
    if version.prerelease.is_empty() {
        return true;
    }
    set.iter()
        .filter_map(|comparator| comparator.semver.as_ref())
        .any(|allowed| {
            !allowed.prerelease.is_empty()
                && (allowed.major, allowed.minor, allowed.patch)
                    == (version.major, version.minor, version.patch)
        })
}

impl Range {
    /// `new Range(range)`.
    ///
    /// # Errors
    ///
    /// What reading a comparator throws (`Invalid comparator: x`, a version's message), or
    /// `Invalid SemVer Range: <raw>`.
    pub fn new(range: &str) -> Result<Self, String> {
        let raw = collapse_blanks(trim(range));
        let mut set = Vec::new();
        for alternative in raw.split("||") {
            let comparators = parse_range(trim(alternative))?;
            if !comparators.is_empty() {
                set.push(comparators);
            }
        }
        if set.is_empty() {
            return Err(format!("Invalid SemVer Range: {raw}"));
        }
        if set.len() > 1 {
            let first = set.first().cloned();
            set.retain(|comparators| !comparators.first().is_some_and(is_null_set));
            if set.is_empty() {
                set.extend(first);
            } else if set.len() > 1 {
                // If we have any that are `*`, then the range is just `*`.
                let any = set
                    .iter()
                    .find(|comparators| matches!(comparators.as_slice(), [only] if is_any(only)));
                if let Some(any) = any.cloned() {
                    set = vec![any];
                }
            }
        }
        Ok(Self { raw, set })
    }

    /// The `range` getter: the sets joined by `||`, each set's comparators by a space.
    #[must_use]
    pub fn range(&self) -> String {
        let sets: Vec<String> = self
            .set
            .iter()
            .map(|comparators| {
                let values: Vec<&str> = comparators
                    .iter()
                    .map(|comparator| trim(&comparator.value))
                    .collect();
                values.join(" ")
            })
            .collect();
        sets.join("||")
    }

    /// `test(version)`: a text that is no version satisfies nothing.
    #[must_use]
    pub fn test(&self, version: &str) -> bool {
        if version.is_empty() {
            return false;
        }
        SemVer::new(version).is_ok_and(|version| self.test_version(&version))
    }

    /// `test(version)` of a parsed version: whether any set holds for it.
    #[must_use]
    pub fn test_version(&self, version: &SemVer) -> bool {
        self.set.iter().any(|set| test_set(set, version))
    }

    /// `intersects(range)`: whether a set of each, both satisfiable, meet comparator by
    /// comparator.
    ///
    /// # Errors
    ///
    /// What a comparator's intersection throws (`Comparator::intersects`).
    pub fn intersects(&self, range: &Self) -> Result<bool, String> {
        for mine in &self.set {
            if !is_satisfiable(mine)? {
                continue;
            }
            for theirs in &range.set {
                if !is_satisfiable(theirs)? {
                    continue;
                }
                if meet(mine, theirs)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

/// Every comparator of `mine` intersects every one of `theirs`.
fn meet(mine: &[Comparator], theirs: &[Comparator]) -> Result<bool, String> {
    for this in mine {
        for other in theirs {
            if !this.intersects(other)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_at_runs_of_blanks_as_javascript_does() {
        assert_eq!(split_blanks(""), [""]);
        assert_eq!(split_blanks(" a  b\u{a0}"), ["", "a", "b", ""]);
        assert_eq!(split_blanks("a"), ["a"]);
        let range = Range::new("^1.2.3 || ~2").unwrap();
        assert_eq!(range.range(), ">=1.2.3 <2.0.0-0||>=2.0.0 <3.0.0-0");
    }
}
