//! A version (`classes/semver.js`, `internal/identifiers.js`): parsed strictly, ordered by its
//! main numbers and then its pre-release, its build ignored.

use std::cmp::Ordering;

use serde::Serialize;

use crate::js::{MAX_SAFE_INTEGER, compare_code_units, number, trim};

use super::re::{self, FULL, MAX_LENGTH, exec};

/// One pre-release identifier: a number below `MAX_SAFE_INTEGER`, else its text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Identifier {
    /// A numeric identifier JavaScript holds as a number.
    Number(u64),
    /// Any other identifier, a numeric one past `MAX_SAFE_INTEGER` included.
    Text(String),
}

/// A parsed version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SemVer {
    /// The text it was read from.
    pub raw: String,
    /// The major number.
    pub major: u64,
    /// The minor number.
    pub minor: u64,
    /// The patch number.
    pub patch: u64,
    /// The pre-release identifiers.
    pub prerelease: Vec<Identifier>,
    /// The build identifiers.
    pub build: Vec<String>,
    /// The version as the package writes it: `major.minor.patch` and `-prerelease`.
    pub version: String,
}

/// `MAX_SAFE_INTEGER` as the whole number it is.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "2^53 - 1, a whole number a u64 holds"
)]
const MAX_SAFE: u64 = MAX_SAFE_INTEGER as u64;

/// `+text` of a run of digits, refused past `MAX_SAFE_INTEGER` with `message`.
fn main_number(digits: &str, message: &str) -> Result<u64, String> {
    // Seventeen digits or more are at least 10^16, past `MAX_SAFE_INTEGER`.
    let value = if digits.len() > 16 {
        None
    } else {
        digits.parse::<u64>().ok()
    };
    value
        .filter(|&value| value <= MAX_SAFE)
        .ok_or_else(|| message.to_owned())
}

/// An identifier of the pre-release: a number when it is digits below `MAX_SAFE_INTEGER`.
fn identifier(id: &str) -> Identifier {
    let digits = !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit());
    let value = (digits && id.len() <= 16)
        .then(|| id.parse::<u64>().ok())
        .flatten();
    match value.filter(|&value| value < MAX_SAFE) {
        Some(value) => Identifier::Number(value),
        None => Identifier::Text(id.to_owned()),
    }
}

impl Identifier {
    /// The identifier as `join('.')` writes it.
    fn text(&self) -> String {
        match self {
            Self::Number(value) => value.to_string(),
            Self::Text(text) => text.clone(),
        }
    }
}

/// `compareIdentifiers(a, b)`: numbers by value, numbers before texts, texts by code units.
fn compare_identifiers(a: &Identifier, b: &Identifier) -> Ordering {
    if let (Identifier::Number(a), Identifier::Number(b)) = (a, b) {
        return a.cmp(b);
    }
    let (a, b) = (a.text(), b.text());
    let numeric = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    match (numeric(&a), numeric(&b)) {
        (true, true) => number(&a)
            .partial_cmp(&number(&b))
            .unwrap_or(Ordering::Equal),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => compare_code_units(&a, &b),
    }
}

impl SemVer {
    /// `new SemVer(version)`.
    ///
    /// # Errors
    ///
    /// The `TypeError` message the package throws: `version is longer than 256 characters`,
    /// `Invalid Version: <version>`, or `Invalid major version` (minor, patch).
    pub fn new(version: &str) -> Result<Self, String> {
        if version.encode_utf16().count() > MAX_LENGTH {
            return Err(format!("version is longer than {MAX_LENGTH} characters"));
        }
        let invalid = || format!("Invalid Version: {version}");
        let found = exec(&FULL, trim(version)).ok_or_else(invalid)?;
        let group = |index: usize| re::group(&found, index).unwrap_or_default();
        let major = main_number(group(1), "Invalid major version")?;
        let minor = main_number(group(2), "Invalid minor version")?;
        let patch = main_number(group(3), "Invalid patch version")?;
        let prerelease = match group(4) {
            "" => Vec::new(),
            text => text.split('.').map(identifier).collect(),
        };
        let build = match group(5) {
            "" => Vec::new(),
            text => text.split('.').map(str::to_owned).collect(),
        };
        let mut parsed = Self {
            raw: version.to_owned(),
            major,
            minor,
            patch,
            prerelease,
            build,
            version: String::new(),
        };
        parsed.version = parsed.format();
        Ok(parsed)
    }

    /// `format()`: the version as the package writes it.
    fn format(&self) -> String {
        let mut version = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if !self.prerelease.is_empty() {
            let ids: Vec<String> = self.prerelease.iter().map(Identifier::text).collect();
            version.push('-');
            version.push_str(&ids.join("."));
        }
        version
    }

    /// `compare(other)`: equal versions are equal whatever their build, else the main numbers
    /// and then the pre-release decide.
    #[must_use]
    pub fn compare(&self, other: &Self) -> Ordering {
        if other.version == self.version {
            return Ordering::Equal;
        }
        self.compare_main(other)
            .then_with(|| self.compare_pre(other))
    }

    fn compare_main(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }

    /// `comparePre(other)`: no pre-release is above any; else identifier by identifier, a
    /// shorter list first when one is a prefix of the other.
    fn compare_pre(&self, other: &Self) -> Ordering {
        match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
            (false, true) => return Ordering::Less,
            (true, false) => return Ordering::Greater,
            (true, true) => return Ordering::Equal,
            (false, false) => {}
        }
        let mut index = 0;
        loop {
            match (self.prerelease.get(index), other.prerelease.get(index)) {
                (None, None) => return Ordering::Equal,
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (Some(a), Some(b)) if a == b => index += 1,
                (Some(a), Some(b)) => return compare_identifiers(a, b),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_a_pre_release_below_the_safe_limit() {
        let version = SemVer::new(" v1.2.3-beta.9007199254740991.7+b.c ").unwrap();
        assert_eq!(version.version, "1.2.3-beta.9007199254740991.7");
        assert_eq!(version.raw, " v1.2.3-beta.9007199254740991.7+b.c ");
        assert_eq!(
            version.prerelease,
            [
                Identifier::Text("beta".to_owned()),
                Identifier::Text("9007199254740991".to_owned()),
                Identifier::Number(7)
            ]
        );
        assert_eq!(SemVer::new("1.2").unwrap_err(), "Invalid Version: 1.2");
        let lower = SemVer::new("1.0.0-alpha.1").unwrap();
        let higher = SemVer::new("1.0.0-alpha.beta").unwrap();
        assert_eq!(lower.compare(&higher), Ordering::Less);
    }
}
