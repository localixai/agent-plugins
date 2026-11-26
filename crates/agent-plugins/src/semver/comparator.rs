//! One comparator of a range (`classes/comparator.js`, `functions/cmp.js`): an operator and a
//! version, or `ANY`, which every version satisfies.

use std::cmp::Ordering;

use crate::js::trim;

use super::range::{Range, split_blanks};
use super::re::COMPARATOR;
use super::version::SemVer;

/// A comparator: `>=1.2.3`, `<2.0.0-0`, `1.2.3` (equal) or `ANY` (the empty text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparator {
    /// `''`, `<`, `>`, `<=` or `>=`; `=` is read as `''`.
    pub operator: String,
    /// The version, `None` for `ANY`.
    pub semver: Option<SemVer>,
    /// The comparator as the package writes it: the operator and the version, `''` for `ANY`.
    pub value: String,
}

/// `cmp(a, op, b)` for the operators a comparator holds.
fn cmp(a: &SemVer, operator: &str, b: &SemVer) -> bool {
    let order = a.compare(b);
    match operator {
        ">" => order == Ordering::Greater,
        ">=" => order != Ordering::Less,
        "<" => order == Ordering::Less,
        "<=" => order != Ordering::Greater,
        _ => order == Ordering::Equal,
    }
}

impl Comparator {
    /// `new Comparator(comp)`: its blanks collapsed, then read as `COMPARATOR`.
    ///
    /// # Errors
    ///
    /// `Invalid comparator: <comp>`, or the version's own message.
    pub fn new(comp: &str) -> Result<Self, String> {
        let comp = split_blanks(trim(comp)).join(" ");
        let found = COMPARATOR
            .as_ref()
            .and_then(|pattern| pattern.captures(&comp))
            .ok_or_else(|| format!("Invalid comparator: {comp}"))?;
        let mut operator = found.get(1).map_or("", |part| part.as_str()).to_owned();
        if operator == "=" {
            operator.clear();
        }
        let semver = match found.get(2).map_or("", |part| part.as_str()) {
            "" => None,
            version => Some(SemVer::new(version)?),
        };
        let value = semver.as_ref().map_or_else(String::new, |semver| {
            format!("{operator}{}", semver.version)
        });
        Ok(Self {
            operator,
            semver,
            value,
        })
    }

    /// `test(version)`: whether `version` is on this comparator's side.
    #[must_use]
    pub fn test(&self, version: &SemVer) -> bool {
        self.semver
            .as_ref()
            .is_none_or(|semver| cmp(version, &self.operator, semver))
    }

    /// `intersects(comp)`: whether some version satisfies both.
    ///
    /// # Errors
    ///
    /// What reading a comparator's own text as a range throws (the package's; it does not
    /// happen for a comparator it made).
    pub fn intersects(&self, comp: &Self) -> Result<bool, String> {
        if self.operator.is_empty() {
            if self.value.is_empty() {
                return Ok(true);
            }
            return Ok(Range::new(&comp.value)?.test(&self.value));
        } else if comp.operator.is_empty() {
            if comp.value.is_empty() {
                return Ok(true);
            }
            let Some(semver) = &comp.semver else {
                return Ok(false);
            };
            return Ok(Range::new(&self.value)?.test_version(semver));
        }
        let (Some(this), Some(other)) = (&self.semver, &comp.semver) else {
            return Ok(false);
        };
        // Special cases where nothing can possibly be lower.
        if self.value.starts_with("<0.0.0") || comp.value.starts_with("<0.0.0") {
            return Ok(false);
        }
        let (mine, theirs) = (self.operator.as_str(), comp.operator.as_str());
        // Same direction increasing (> or >=), or decreasing (< or <=).
        if mine.starts_with('>') && theirs.starts_with('>')
            || mine.starts_with('<') && theirs.starts_with('<')
        {
            return Ok(true);
        }
        // Same version and both sides are inclusive (<= or >=).
        if this.version == other.version && mine.contains('=') && theirs.contains('=') {
            return Ok(true);
        }
        // Opposite directions, less than or greater than.
        Ok(
            cmp(this, "<", other) && mine.starts_with('>') && theirs.starts_with('<')
                || cmp(this, ">", other) && mine.starts_with('<') && theirs.starts_with('>'),
        )
    }
}
