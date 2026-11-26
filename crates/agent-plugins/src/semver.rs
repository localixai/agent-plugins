//! Semantic versions and ranges as the npm package `semver` 7.8.6 reads them, which the
//! package's dependency ranges and release tags are read by (`src/marketplace/semver.ts`): a
//! version parsed, two compared, a range parsed into its comparators (hyphen ranges, `~`, `^`,
//! x-ranges, `*`, `||`), a version tested against one, and two ranges intersected. It is ported
//! by hand because no Rust crate answers as npm does on the oracle below (2026-11-26):
//! `semver` 1.0.28 reads Cargo's grammar (no blanks around a version, `1.2` alone as `^1.2`,
//! `0 1.2` refused); `nodejs-semver` 4.2.0 and `node-semver` 2.2.0 read leading zeros (`01.2.3`),
//! ranges npm refuses (`.x || >=2.5.0`, `- 2`) and intersect by another rule, and `node-semver`
//! panics on 52 of the oracle's ranges (`~x.X.*`). Each differs on hundreds of its cases.
//!
//! Only what `marketplace/semver.ts` asks is here, with the default options it passes: strict
//! parsing (no `loose`) and pre-releases matched only by a comparator that names one (no
//! `includePrerelease`). The patterns are the package's "safe" ones, their `\s*`, `\d+` and the
//! like bounded as it bounds them (`re.rs`), run by `js::reg_exp` with JavaScript's classes. A
//! text is read as JavaScript reads it: its whitespace is `js::is_whitespace`'s and its positions
//! UTF-16 code units. `tests/differential/semver.rs` holds it to what Bun answered
//! (`scripts/rust/js-oracle/semver.ts`).
//!
//! The package is ISC licensed; this port keeps its notice:
//!
//! Copyright (c) Isaac Z. Schlueter and Contributors
//!
//! Permission to use, copy, modify, and/or distribute this software for any purpose with or
//! without fee is hereby granted, provided that the above copyright notice and this permission
//! notice appear in all copies.
//!
//! THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS
//! SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE
//! AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//! WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT,
//! NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
//! PERFORMANCE OF THIS SOFTWARE.

mod comparator;
mod range;
mod re;
mod replace;
mod trim;
mod version;

pub use comparator::Comparator;
pub use range::Range;
pub use version::{Identifier, SemVer};

use std::cmp::Ordering;

/// `semver.parse(version)`: the version, or `None` (`null`) for a text that is none.
#[must_use]
pub fn parse(version: &str) -> Option<SemVer> {
    SemVer::new(version).ok()
}

/// `semver.validRange(range)`: the range as the package writes it (`*` for any version), or
/// `None` (`null`) for a text that is no range.
#[must_use]
pub fn valid_range(range: &str) -> Option<String> {
    let range = Range::new(range).ok()?.range();
    Some(if range.is_empty() {
        "*".to_owned()
    } else {
        range
    })
}

/// `semver.satisfies(version, range)`: an invalid range is satisfied by nothing.
#[must_use]
pub fn satisfies(version: &str, range: &str) -> bool {
    Range::new(range).is_ok_and(|range| range.test(version))
}

/// `semver.maxSatisfying(versions, range)`: the highest of `versions` inside `range`, the first
/// of equal ones; `None` (`null`) when none is, or the range is invalid.
#[must_use]
pub fn max_satisfying(versions: &[String], range: &str) -> Option<String> {
    let range = Range::new(range).ok()?;
    let mut max: Option<(&String, SemVer)> = None;
    for version in versions {
        if !range.test(version) {
            continue;
        }
        let Ok(parsed) = SemVer::new(version) else {
            continue;
        };
        if max
            .as_ref()
            .is_none_or(|(_, highest)| highest.compare(&parsed) == Ordering::Less)
        {
            max = Some((version, parsed));
        }
    }
    max.map(|(version, _)| version.clone())
}

/// `semver.compare(a, b)`: `-1`, `0` or `1`.
///
/// # Errors
///
/// The message the package throws for a text that is no version (`Invalid Version: x`).
pub fn compare(a: &str, b: &str) -> Result<i8, String> {
    let first = SemVer::new(a)?;
    let second = SemVer::new(b)?;
    Ok(first.compare(&second) as i8)
}

/// `semver.rcompare(a, b)`: `compare(b, a)`, so a sort by it puts the highest first.
///
/// # Errors
///
/// As `compare`: the first of `b` and `a` that is no version.
pub fn rcompare(a: &str, b: &str) -> Result<i8, String> {
    compare(b, a)
}

/// `semver.intersects(r1, r2)`: whether some version satisfies both ranges.
///
/// # Errors
///
/// The message the package throws for a range that does not parse.
pub fn intersects(r1: &str, r2: &str) -> Result<bool, String> {
    let first = Range::new(r1)?;
    let second = Range::new(r2)?;
    first.intersects(&second)
}
