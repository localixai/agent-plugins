//! Port of `src/convert/to-claude-code/manifest.ts`, switched.
//!
//! `.claude-plugin/plugin.json` from a core manifest (Agent Plugins Extended §18.4.2 item 1): the
//! nine shared fields copied, `$schema` dropped, `extensions` reported. An `author` without `name` and
//! a `homepage` that is no URL fail in Claude Code (§2.2): each is left out and reported, never
//! invented. The name is never changed (§18.4.1 item 2): one `claude plugin validate` refuses (a
//! reserved name) stops the conversion, since its output could not pass the validator (item 3;
//! judgment, spec/judgments.md), and its warnings (not kebab-case, J18.19) are reported.

use serde::Serialize;

use crate::convert::notes::{ConversionNote, not_converted, reported, stop};
use crate::format::core::manifest::CoreManifest;
use crate::format::issues::Severity;
use crate::format::names::name_validation_issues;
use crate::js::url;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};

/// Where the converted manifest is written.
pub const CLAUDE_MANIFEST: &str = ".claude-plugin/plugin.json";
const SOURCE: &str = "plugin.json";

fn name_notes(name: &str) -> Vec<ConversionNote> {
    name_validation_issues(name, false, None)
        .into_iter()
        .map(|issue| {
            let note = match issue.severity {
                Severity::Error => stop,
                Severity::Warning => reported,
            };
            note(SOURCE.to_owned(), Some("name".to_owned()), issue.message)
        })
        .collect()
}

/// The nine fields both manifests define, in the order the converter writes them (§18.3.1).
pub const SHARED_FIELDS: [&str; 9] = [
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

/// The value written and what to say about it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ManifestPart {
    /// The manifest's members.
    pub value: JsObject,
    /// What was not carried over.
    pub notes: Vec<ConversionNote>,
}

/// The fields Claude Code refuses as the core lets them be, each with why it is left out.
fn refused_fields(manifest: &CoreManifest) -> Vec<(&'static str, String)> {
    let mut refused = Vec::new();
    if manifest
        .author
        .as_ref()
        .is_some_and(|author| author.name.is_none())
    {
        refused.push((
            "author",
            "author has no \"name\", which Claude Code requires: left out".to_owned(),
        ));
    }
    if let Some(homepage) = &manifest.homepage
        && url::parse(homepage).is_none()
    {
        refused.push((
            "homepage",
            format!("homepage \"{homepage}\" is not a URL, which Claude Code requires: left out"),
        ));
    }
    refused
}

/// `.claude-plugin/plugin.json` from a core manifest.
#[must_use]
pub fn claude_code_manifest(manifest: &CoreManifest) -> ManifestPart {
    let refused = refused_fields(manifest);
    let mut notes = name_notes(&manifest.name);
    notes.extend(refused.iter().map(|(field, message)| {
        not_converted(
            SOURCE.to_owned(),
            Some((*field).to_owned()),
            message.clone(),
        )
    }));
    notes.extend(manifest.extensions.iter().flat_map(|extensions| {
        extensions.keys().map(|namespace| {
            not_converted(
                SOURCE.to_owned(),
                Some(format!("extensions.{namespace}")),
                "client extension data has no Claude Code counterpart".to_owned(),
            )
        })
    }));
    let written = to_js_value(manifest).unwrap_or(JsValue::Undefined);
    let value = SHARED_FIELDS
        .iter()
        .filter(|field| !refused.iter().any(|(name, _)| name == *field))
        .filter_map(|field| {
            written
                .get(field)
                .filter(|member| !member.is_undefined())
                .map(|member| ((*field).to_owned(), member.clone()))
        })
        .collect();
    ManifestPart { value, notes }
}

crate::registry::exports! {
    "convert/to-claude-code/manifest" {
        "CLAUDE_MANIFEST" => ok || CLAUDE_MANIFEST;
        "SHARED_FIELDS" => ok || SHARED_FIELDS;
        "claudeCodeManifest" => ok |manifest: CoreManifest| claude_code_manifest(&manifest);
    }
}
