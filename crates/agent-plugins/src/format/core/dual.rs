//! Port of `src/format/core/dual.ts`, switched.
//!
//! The manifest half of a dual package (Agent Plugins Extended §18.3.1): `plugin.json` (core) and
//! `.claude-plugin/plugin.json` (Claude Code) with the same values in the fields both define, chosen
//! so that they load in both. The check reports, per file, what loads in one form only: a core
//! manifest that fails core §5; a field whose two values differ; a name that is not kebab-case (a `.`
//! draws Claude Code's warning, J18.19); an `author` without `name` and a `homepage` that is no
//! absolute URL, which Claude Code refuses; a missing `description`, which its validator warns
//! about; a `version` that is not SemVer; `$schema` or `extensions` in the Claude Code manifest.
//! Claude Code's own rules on its manifest stay `validatePlugin`'s.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use super::manifest::read_core_manifest;
use crate::format::canonical_json::canonical_json;
use crate::format::issues::{PluginIssue, Severity};
use crate::format::validate::finding::{ValidationIssue, error_in, warning_in};
use crate::js::{JsObject, JsValue, Pattern, regex_test};
use crate::protocol::is_record::is_record;

/// The core manifest's file.
pub const CORE_MANIFEST_FILE: &str = "plugin.json";
/// The Claude Code manifest's file.
pub const CLAUDE_MANIFEST_FILE: &str = ".claude-plugin/plugin.json";

/// The fields both forms define (§18.3.1).
const SHARED: [&str; 9] = [
    "name",
    "displayName",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
];

static KEBAB: Pattern = LazyLock::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").ok());
static SEMVER: Pattern = LazyLock::new(|| {
    Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$").ok()
});

/// A finding of the core manifest's own reader, with the file it is about added to the end
/// (`{ ...found, file }`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CoreIssue {
    /// How much it weighs.
    pub severity: Severity,
    /// The place in the file.
    pub path: String,
    /// What is wrong.
    pub message: String,
    /// The advice, when the reader gave one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<String>,
    /// The file.
    pub file: String,
}

/// One finding of the check: the core manifest reader's, or the check's own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum DualIssue {
    /// A finding of `read_core_manifest`.
    Core(CoreIssue),
    /// A finding of the check.
    Found(ValidationIssue),
}

fn is_absolute_url(text: &str) -> bool {
    crate::js::url::parse(text).is_some()
}

fn differences(core: &JsObject, claude: &JsObject) -> Vec<ValidationIssue> {
    let value = |members: &'_ JsObject, field: &str| {
        canonical_json(members.get(field).unwrap_or(&JsValue::Undefined))
    };
    SHARED
        .iter()
        .filter(|field| {
            (core.contains_key(**field) || claude.contains_key(**field))
                && value(core, field) != value(claude, field)
        })
        .map(|field| {
            error_in(
                CORE_MANIFEST_FILE,
                field,
                &format!(
                    "\"{field}\" differs from {CLAUDE_MANIFEST_FILE}: a dual package gives both the same value"
                ),
            )
        })
        .collect()
}

/// What of the shared values loads in one form only, judged on the core manifest's values.
fn value_issues(core: &JsObject) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let text = |field: &str| core.get(field).and_then(JsValue::as_str);
    if let Some(name) = text("name")
        && !regex_test(&KEBAB, name)
    {
        issues.push(warning_in(
            CORE_MANIFEST_FILE,
            "name",
            &format!("\"{name}\" is not kebab-case: Claude Code accepts it with a warning"),
        ));
    }
    if let Some(author) = core.get("author")
        && is_record(author)
        && !author.get("name").is_some_and(JsValue::is_string)
    {
        issues.push(error_in(
            CORE_MANIFEST_FILE,
            "author",
            "Claude Code requires \"author.name\"",
        ));
    }
    if let Some(homepage) = text("homepage")
        && !is_absolute_url(homepage)
    {
        issues.push(error_in(
            CORE_MANIFEST_FILE,
            "homepage",
            "Claude Code requires \"homepage\" to be an absolute URL",
        ));
    }
    if core.get("description").is_none_or(JsValue::is_undefined) {
        issues.push(warning_in(
            CORE_MANIFEST_FILE,
            "description",
            "Both forms should carry a \"description\"",
        ));
    }
    if let Some(version) = text("version")
        && !regex_test(&SEMVER, version)
    {
        issues.push(warning_in(
            CORE_MANIFEST_FILE,
            "version",
            &format!("\"{version}\" is not Semantic Versioning"),
        ));
    }
    issues
}

fn claude_only_issues(claude: &JsObject) -> Vec<ValidationIssue> {
    ["$schema", "extensions"]
        .into_iter()
        .filter(|field| claude.contains_key(*field))
        .map(|field| {
            warning_in(
                CLAUDE_MANIFEST_FILE,
                field,
                &format!("\"{field}\" belongs in {CORE_MANIFEST_FILE} only"),
            )
        })
        .collect()
}

fn core_issue(found: PluginIssue) -> DualIssue {
    DualIssue::Core(CoreIssue {
        severity: found.severity,
        path: found.path,
        message: found.message,
        guidance: found.guidance,
        file: CORE_MANIFEST_FILE.to_owned(),
    })
}

/// Checks the two parsed manifests of a dual package (§18.3.1).
#[must_use]
pub fn check_dual_manifests(core: &JsValue, claude: &JsValue) -> Vec<DualIssue> {
    let read = read_core_manifest(core, &[]);
    let mut issues: Vec<DualIssue> = read.issues.into_iter().map(core_issue).collect();
    let (JsValue::Object(core), JsValue::Object(claude)) = (core, claude) else {
        let which = if is_record(claude) {
            CORE_MANIFEST_FILE
        } else {
            CLAUDE_MANIFEST_FILE
        };
        issues.push(DualIssue::Found(error_in(
            which,
            "",
            &format!("{which} is not a JSON object"),
        )));
        return issues;
    };
    issues.extend(
        value_issues(core)
            .into_iter()
            .chain(differences(core, claude))
            .chain(claude_only_issues(claude))
            .map(DualIssue::Found),
    );
    issues
}

crate::registry::exports! {
    "format/core/dual" {
        "CORE_MANIFEST_FILE" => ok || CORE_MANIFEST_FILE;
        "CLAUDE_MANIFEST_FILE" => ok || CLAUDE_MANIFEST_FILE;
        "checkDualManifests" => ok |core: JsValue, claude: JsValue| check_dual_manifests(&core, &claude);
    }
}
