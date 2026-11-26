//! Port of `src/format/marketplace/plugin-source-fields.ts`, switched.
//!
//! The field checks every object plugin source shares (plugins/marketplace-reference.md, "Plugin
//! sources" and "Invalid input on a source"): a known type "with a required field missing or of the
//! wrong type" is `Invalid input` on the entry's `source`.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::format::path_text::PathPlatform;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, Pattern, code_unit_length, regex_test};

/// How a marketplace is being read (`ReadMode` of `src/format/marketplace/types.ts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReadMode {
    /// Loading it.
    Load,
    /// Validating it.
    Validate,
}

/// Where a source is being read: the path its issues name, and what bare names and `\` depend on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSite {
    /// The path the issues name.
    pub path: String,
    /// What a bare name resolves under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_root: Option<String>,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// `validate` leaves out what only installing or loading refuses (a backslash path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ReadMode>,
}

/// `Invalid input` on the source.
#[must_use]
pub fn invalid_input(site: &SourceSite) -> Vec<PluginIssue> {
    vec![error(&site.path, "Invalid input")]
}

/// A full 40-character lowercase commit SHA ("Plugin sources": `sha`).
static COMMIT_SHA: Pattern = LazyLock::new(|| Regex::new(r"^[0-9a-f]{40}$").ok());
const SHA_LENGTH: usize = 40;
/// The archive's digest: 64 hex characters, uppercase or lowercase ("archive plugin source").
static ARCHIVE_DIGEST: Pattern = LazyLock::new(|| Regex::new(r"^[0-9A-Fa-f]{64}$").ok());

/// A source as written: an object of unknown members.
pub type RawSource = JsObject;

/// A non-empty string.
#[must_use]
pub fn is_text(value: &JsValue) -> bool {
    value.as_str().is_some_and(|text| !text.is_empty())
}

fn is_string(value: &JsValue) -> bool {
    value.is_string()
}

/// An optional field: `value` is `None` when the key is absent. A present value of the wrong type is no value at all.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Optional {
    /// The text, when the key is there.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// A value that may be `undefined`, which an object leaves out.
pub trait Absent {
    /// Whether the value is `undefined`.
    fn is_absent(&self) -> bool;
}

impl<T> Absent for Option<T> {
    fn is_absent(&self) -> bool {
        self.is_none()
    }
}

impl Absent for GitPin {
    fn is_absent(&self) -> bool {
        false
    }
}

/// A source of the right shape, or the issues of its fields. A field present, of the right type and
/// breaking its rule is reported on that field (spec §15.6.7: "a field present with an invalid value
/// names that field"); the wording is oracle 2.1.325 (spec/judgments.md).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(bound(serialize = "T: Serialize + Absent"))]
pub struct Checked<T> {
    /// What was read; left out of the answer where it is `undefined`.
    #[serde(skip_serializing_if = "Absent::is_absent")]
    pub value: T,
    /// What was found wrong.
    pub issues: Vec<PluginIssue>,
}

/// `issues` when there are any, else the source.
#[must_use]
pub fn refined(source: &JsValue, issues: &[PluginIssue]) -> JsValue {
    if issues.is_empty() {
        source.clone()
    } else {
        to_js_value(&issues).unwrap_or_default()
    }
}

/// An issue on one field of the source.
#[must_use]
pub fn field_error(site: &SourceSite, key: &str, message: &str) -> PluginIssue {
    error(&format!("{}.{key}", site.path), message)
}

/// A required text field: the text, or `None` when the key is missing, empty or not a string.
#[must_use]
pub fn required(raw: &RawSource, key: &str) -> Option<String> {
    raw.get(key)
        .filter(|value| is_text(value))
        .and_then(JsValue::as_str)
        .map(str::to_owned)
}

fn optional(raw: &RawSource, key: &str, accepts: fn(&JsValue) -> bool) -> Option<Optional> {
    let value = raw.get(key).unwrap_or(&JsValue::Undefined);
    if value.is_undefined() {
        return Some(Optional { value: None });
    }
    if accepts(value) {
        Some(Optional {
            value: value.as_str().map(str::to_owned),
        })
    } else {
        None
    }
}

/// An optional non-empty text field.
#[must_use]
pub fn optional_text(raw: &RawSource, key: &str) -> Option<Optional> {
    optional(raw, key, is_text)
}

/// `sha256`: a string, else `None`; one that is not 64 hex characters is an issue on the field.
#[must_use]
pub fn optional_digest(raw: &RawSource, site: &SourceSite) -> Option<Checked<Option<String>>> {
    let digest = optional(raw, "sha256", is_string)?;
    let broken = digest
        .value
        .as_deref()
        .is_some_and(|text| !regex_test(&ARCHIVE_DIGEST, text));
    Some(Checked {
        value: digest.value,
        issues: if broken {
            vec![field_error(
                site,
                "sha256",
                "Must be a 64-character hex SHA-256 digest",
            )]
        } else {
            Vec::new()
        },
    })
}

/// A `sha` of the wrong length is also reported as too short or too long, before the SHA message.
fn sha_issues(sha: &str, site: &SourceSite) -> Vec<PluginIssue> {
    if regex_test(&COMMIT_SHA, sha) {
        return Vec::new();
    }
    let length = code_unit_length(sha);
    let mut issues = Vec::new();
    if length != SHA_LENGTH {
        issues.push(field_error(
            site,
            "sha",
            &format!(
                "Too {}: expected string to have exactly {SHA_LENGTH} characters",
                if length < SHA_LENGTH { "small" } else { "big" }
            ),
        ));
    }
    issues.push(field_error(
        site,
        "sha",
        "Must be a full 40-character lowercase git commit SHA",
    ));
    issues
}

/// The `ref` and `sha` of a git source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GitPin {
    /// The branch or tag.
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// The commit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
}

/// The `ref` and `sha` the git sources share, or `None` when either is present and of the wrong type.
#[must_use]
pub fn read_git_pin(raw: &RawSource, site: &SourceSite) -> Option<Checked<GitPin>> {
    let reference = optional_text(raw, "ref")?;
    let sha = optional(raw, "sha", is_string)?;
    let issues = sha
        .value
        .as_deref()
        .map_or_else(Vec::new, |text| sha_issues(text, site));
    Some(Checked {
        value: GitPin {
            reference: reference.value,
            sha: sha.value,
        },
        issues,
    })
}

crate::registry::exports! {
    "format/marketplace/plugin-source-fields" {
        "invalidInput" => ok |site: SourceSite| invalid_input(&site);
    }
}
