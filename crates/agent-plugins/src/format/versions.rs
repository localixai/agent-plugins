//! Port of `src/format/versions.ts`, switched.
//!
//! Host release numbers, as the docs' "since v2.1.N" rules name them: one comparison for every
//! such rule. A leading `v`, surrounding blanks and text after the number
//! (`2.1.320 (Claude Code)`) are tolerated; a pre-release (`2.1.310-beta.1`) sorts before its
//! release, its dot-separated parts compared numerically when both are numbers, as in semver. A
//! text with no leading `major.minor.patch` is no version (judgment, spec/judgments.md). The
//! order is node-semver's precedence; plugin version ranges are `marketplace/semver`'s.

use std::cmp::Ordering;

use crate::js::trim;

/// `Number.MAX_SAFE_INTEGER`: node-semver's largest main version part.
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// node-semver's longest version text.
const MAX_LENGTH: usize = 256;

/// A version as node-semver's `parse` reads it: build metadata never occurs here, as the text
/// `release_of` hands over has none.
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<String>,
}

/// One main version part: `0|[1-9]\d*`, no larger than a safe integer.
fn main_part(text: &str) -> Option<u64> {
    let plain = !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    if !plain {
        return None;
    }
    text.parse::<u64>()
        .ok()
        .filter(|value| *value <= MAX_SAFE_INTEGER)
}

/// One pre-release identifier: `\d*[a-zA-Z-][a-zA-Z0-9-]*` or `0|[1-9]\d*`.
fn is_prerelease_identifier(text: &str) -> bool {
    let characters = !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    let numeric = text.bytes().all(|byte| byte.is_ascii_digit());
    characters && (!numeric || text == "0" || !text.starts_with('0'))
}

/// node-semver's `parse` of a `major.minor.patch[-pre]` text; `None` where it gives `null`.
fn parse(version: &str) -> Option<SemVer> {
    if version.len() > MAX_LENGTH {
        return None;
    }
    let (main, prerelease) = match version.split_once('-') {
        Some((main, prerelease)) => (main, Some(prerelease)),
        None => (version, None),
    };
    let mut parts = main.split('.');
    let major = main_part(parts.next()?)?;
    let minor = main_part(parts.next()?)?;
    let patch = main_part(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    let prerelease: Vec<String> = match prerelease {
        None => Vec::new(),
        Some(text) if text.split('.').all(is_prerelease_identifier) => {
            text.split('.').map(str::to_owned).collect()
        }
        Some(_) => return None,
    };
    Some(SemVer {
        major,
        minor,
        patch,
        prerelease,
    })
}

/// `^v?(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)` on the trimmed text: the `major.minor.patch[-pre]` it
/// starts with.
fn release_text(text: &str) -> Option<&str> {
    let trimmed = trim(text);
    let rest = trimmed.strip_prefix('v').unwrap_or(trimmed);
    let mut end = 0;
    for part in 0..3 {
        if part > 0 {
            if rest.as_bytes().get(end) != Some(&b'.') {
                return None;
            }
            end += 1;
        }
        let digits = rest
            .bytes()
            .skip(end)
            .take_while(u8::is_ascii_digit)
            .count();
        if digits == 0 {
            return None;
        }
        end += digits;
    }
    if rest.as_bytes().get(end) == Some(&b'-') {
        let tail = rest
            .bytes()
            .skip(end + 1)
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
            .count();
        if tail > 0 {
            end += 1 + tail;
        }
    }
    rest.get(..end)
}

/// The `major.minor.patch[-pre]` a text starts with, as node-semver reads it; none for a text
/// with none.
fn release_of(text: &str) -> Option<SemVer> {
    parse(release_text(text)?)
}

/// node-semver's `compareIdentifiers` of two pre-release identifiers: numbers by value, before
/// words; words by code unit.
fn compare_identifiers(a: &str, b: &str) -> Ordering {
    let numeric = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    match (numeric(a), numeric(b)) {
        (true, true) => a
            .parse::<f64>()
            .unwrap_or(f64::NAN)
            .partial_cmp(&b.parse::<f64>().unwrap_or(f64::NAN))
            .unwrap_or(Ordering::Equal),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.cmp(b),
    }
}

/// node-semver's `compare`: the main numbers, then the pre-release (a release is after its
/// pre-releases, a longer list after its prefix).
fn compare(left: &SemVer, right: &SemVer) -> Ordering {
    let main = left
        .major
        .cmp(&right.major)
        .then(left.minor.cmp(&right.minor))
        .then(left.patch.cmp(&right.patch));
    if main != Ordering::Equal {
        return main;
    }
    match (left.prerelease.is_empty(), right.prerelease.is_empty()) {
        (false, true) => return Ordering::Less,
        (true, false) => return Ordering::Greater,
        (true, true) => return Ordering::Equal,
        (false, false) => {}
    }
    for (a, b) in left.prerelease.iter().zip(&right.prerelease) {
        if a != b {
            return compare_identifiers(a, b);
        }
    }
    left.prerelease.len().cmp(&right.prerelease.len())
}

/// How `a` is placed before, the same as or after `b`; `None` when either is no version.
#[must_use]
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    let (left, right) = (release_of(a)?, release_of(b)?);
    Some(compare(&left, &right))
}

/// Whether `version` is `minimum` or later; a text that is no version is never at least
/// anything.
#[must_use]
pub fn version_at_least(version: &str, minimum: &str) -> bool {
    compare_versions(version, minimum).is_some_and(Ordering::is_ge)
}

crate::registry::exports! {
    "format/versions" {
        "compareVersions" => ok |a: String, b: String| compare_versions(&a, &b).map(crate::registry::sign);
        "versionAtLeast" => ok |version: String, minimum: String| version_at_least(&version, &minimum);
    }
}
