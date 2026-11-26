//! Port of `src/format/core/versions.ts`, switched.
//!
//! The core's schema identifiers (Agent Plugins 1.1.0, core §5.2, §7.2.1, §7.3.1, §10.1): every
//! document of a release names its schema by a canonical identifier that carries the release's
//! version, `https://agent-plugins.org/schemas/<version>/<document>.schema.json`. The kit selects its
//! local rules from that identifier and never retrieves a schema.

use std::sync::LazyLock;

use crate::js::JsValue;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::Pattern;
use crate::protocol::is_record::is_record;

/// The documents the core defines a schema for (core §10.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoreDocument {
    /// `plugin`
    Plugin,
    /// `mcp`
    Mcp,
    /// `hooks`
    Hooks,
}

impl CoreDocument {
    /// The document's name as the identifier spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Mcp => "mcp",
            Self::Hooks => "hooks",
        }
    }
}

/// The three documents the core defines a schema for (core §10.1).
pub const CORE_DOCUMENTS: [CoreDocument; 3] =
    [CoreDocument::Plugin, CoreDocument::Mcp, CoreDocument::Hooks];

/// An Agent Plugins version the kit implements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoreVersion {
    /// `1.1.0`
    #[serde(rename = "1.1.0")]
    V1_1_0,
}

impl CoreVersion {
    /// The version as its string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1_1_0 => "1.1.0",
        }
    }
}

/// The Agent Plugins versions the kit implements. 1.0.0 is not mapped to 1.1.0: its manifest has no
/// `displayName`, its servers no `extensions` and it has no `hooks.json`, so the kit does not
/// recognise the two as compatible (core §5.2 allows a mapping only when it is explicit).
pub const CORE_VERSIONS: [CoreVersion; 1] = [CoreVersion::V1_1_0];

/// A canonical identifier: the version is any SemVer, so an unsupported release is still
/// recognised as core. Digits are ASCII (RS6).
static IDENTIFIER: Pattern = LazyLock::new(|| {
    Regex::new(
        r"^https://agent-plugins\.org/schemas/([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)/(plugin|mcp|hooks)\.schema\.json$",
    )
    .ok()
});

/// The canonical identifier of a document of a release.
#[must_use]
pub fn core_schema_id(version: CoreVersion, document: CoreDocument) -> String {
    format!(
        "https://agent-plugins.org/schemas/{}/{}.schema.json",
        version.as_str(),
        document.as_str()
    )
}

/// The version a `$schema` value names for `document`, or `None` when it is no canonical
/// identifier of it.
#[must_use]
pub fn core_schema_version(value: &JsValue, document: CoreDocument) -> Option<String> {
    let JsValue::String(text) = value else {
        return None;
    };
    let captures = IDENTIFIER.as_ref()?.captures(text)?;
    if captures.get(2)?.as_str() == document.as_str() {
        Some(captures.get(1)?.as_str().to_owned())
    } else {
        None
    }
}

/// The implemented version a version string is, or `None` when the kit does not implement it.
#[must_use]
pub fn supported_core_version(version: Option<&str>) -> Option<CoreVersion> {
    CORE_VERSIONS
        .iter()
        .copied()
        .find(|known| Some(known.as_str()) == version)
}

/// Whether a root `plugin.json` value makes the folder a core package (spec §1.3): a JSON object whose
/// `$schema` is a canonical core manifest identifier of any release (Judgment J1.1: anything else is
/// an ordinary file of a Claude Code plugin).
#[must_use]
pub fn is_core_manifest_value(raw: &JsValue) -> bool {
    is_record(raw)
        && core_schema_version(
            raw.get("$schema").unwrap_or(&JsValue::Null),
            CoreDocument::Plugin,
        )
        .is_some()
}

crate::registry::exports! {
    "format/core/versions" {
        "CORE_DOCUMENTS" => ok || CORE_DOCUMENTS;
        "CORE_VERSIONS" => ok || CORE_VERSIONS;
        "coreSchemaId" => ok |version: CoreVersion, document: CoreDocument| core_schema_id(version, document);
        "coreSchemaVersion" => ok |value: JsValue, document: CoreDocument| core_schema_version(&value, document);
        "supportedCoreVersion" => ok |version: Option<String>| supported_core_version(version.as_deref());
        "isCoreManifestValue" => ok |raw: JsValue| is_core_manifest_value(&raw);
    }
}
