//! Port of `src/format/validate/plugin-manifest.ts`, switched.
//!
//! `claude plugin validate` on a parsed `plugin.json` (oracle 2.1.319): what is wrong with each
//! key's type, the name, the keys it does not know, and what is missing. Whether the paths it
//! names exist is for the caller, which has the files: `claims` lists them.
//!
//! The validator parses the manifest against its schema first. A manifest that fails it is still
//! checked for unknown keys and paths, but not for what is missing or how its name reads, and the
//! hooks and MCP servers written inline are not read at all: `inline` is empty then.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::format::issues::Severity;
use crate::format::names::NameContext;
use crate::format::validate::finding::{Finding, error_at, warning_at};
use crate::format::validate::hooks_check::{HooksSite, hooks_map_findings};
use crate::format::validate::inline_sources::source_findings;
use crate::format::validate::mcp_check::inline_findings;
use crate::format::validate::path_claims::{PathClaim, claims_of};
use crate::format::validate::plugin_fields::{
    SUGGESTED_KEYS, field_findings, loose_object_findings, missing_name, plugin_keys,
};
use crate::format::validate::plugin_name::{NameFindings, NameOptions, name_findings};
use crate::format::validate::plugin_paths::{PathKey, is_path_key};
use crate::format::validate::unknown_fields::{FieldSite, unknown_field_finding};
use crate::js::{JsObject, JsValue};

/// What the checks of one manifest found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ManifestChecks {
    /// The findings of the manifest itself.
    pub findings: Vec<Finding>,
    /// What the inline `hooks` and `mcpServers` hold; for the plugin's own file, not for a marketplace entry.
    pub inline: Vec<Finding>,
    /// The paths it claims exist.
    pub claims: Vec<PathClaim>,
}

/// What changes the checks of a manifest (`NameOptions` and the entry's name).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestOptions {
    /// An official marketplace lists this plugin: its name is not checked against the reserved ones.
    #[serde(default)]
    pub official: Option<bool>,
    /// Where the manifest is read: a marketplace entry's gets the shorter advice.
    #[serde(default)]
    pub context: Option<NameContext>,
    /// The name of the marketplace entry this manifest belongs to: a name it already reported is not reported twice.
    #[serde(default)]
    pub entry_name: Option<String>,
}

/// Fields that are advised on when missing or empty (oracle 2.1.319).
const RECOMMENDED: [(&str, &str); 3] = [
    (
        "version",
        "No version specified. Consider adding a version following semver (e.g., \"1.0.0\")",
    ),
    (
        "description",
        "No description provided. Adding a description helps users understand what your plugin does",
    ),
    (
        "author",
        "No author information provided. Consider adding author details for plugin attribution",
    ),
];

const NO_LONGER_READ: &str = "'capabilities' is no longer read: what a hooks module hooks and calls on $ is read from its source and listed under its hooks.json below. Delete the field.";

const EXPERIMENTAL_KEYS: [&str; 3] = ["themes", "monitors", "evals"];
const MOVED_UNDER_EXPERIMENTAL: [&str; 2] = ["themes", "monitors"];

fn moved_warning(key: &str) -> Finding {
    warning_at(
        key,
        &format!(
            "'{key}' is an experimental component; declare it under 'experimental.{key}' instead of at the top level. Top-level still loads for now but will be removed in a future release."
        ),
    )
}

/// The path key `key` names, if it names one.
fn path_key(key: &str) -> Option<PathKey> {
    PathKey::deserialize(JsValue::from(key)).ok()
}

/// The shape findings of one key under `experimental` (`types` is not one of them).
fn experimental_path(key: &str, value: &JsValue) -> Vec<Finding> {
    match path_key(key) {
        Some(found) if key != "types" => {
            source_findings(found, &format!("experimental.{key}"), value)
        }
        _ => Vec::new(),
    }
}

fn experimental_findings(experimental: Option<&JsValue>) -> Vec<Finding> {
    let Some(JsValue::Object(experimental)) = experimental else {
        return Vec::new();
    };
    let known: Vec<String> = EXPERIMENTAL_KEYS
        .iter()
        .map(|key| (*key).to_owned())
        .collect();
    experimental
        .iter()
        .flat_map(|(key, value)| {
            if EXPERIMENTAL_KEYS.contains(&key.as_str()) {
                experimental_path(key, value)
            } else {
                let path = format!("experimental.{key}");
                vec![unknown_field_finding(
                    key,
                    &known,
                    &path,
                    FieldSite::Manifest,
                )]
            }
        })
        .collect()
}

/// The path rules of a manifest whose keys are well typed: each path key's shape (`./`, the
/// extension), at the top level and under `experimental`, and the `..` refusal. Loading reports
/// these as the validator does ("Claude Code reports the same problem when it loads the plugin").
#[must_use]
pub fn manifest_path_findings(manifest: &JsObject) -> Vec<Finding> {
    let nested: Vec<Finding> = match manifest.get("experimental") {
        Some(JsValue::Object(experimental)) => experimental
            .iter()
            .flat_map(|(key, value)| experimental_path(key, value))
            .collect(),
        _ => Vec::new(),
    };
    let top = manifest
        .iter()
        .flat_map(|(key, value)| match path_key(key) {
            Some(found) if !value.is_undefined() => source_findings(found, key, value),
            _ => Vec::new(),
        });
    top.chain(nested)
        .chain(claims_of(manifest).findings)
        .collect()
}

fn unknown_keys(manifest: &JsObject) -> Vec<Finding> {
    let keys = plugin_keys();
    let suggested: Vec<String> = SUGGESTED_KEYS.iter().map(|key| (*key).to_owned()).collect();
    manifest
        .keys()
        .filter(|key| !keys.contains(&key.as_str()) && !is_path_key(key) && *key != "capabilities")
        .map(|key| unknown_field_finding(key, &suggested, key, FieldSite::Manifest))
        .collect()
}

/// Whether `key` is written: `undefined` is not.
fn written(manifest: &JsObject, key: &str) -> bool {
    manifest.get(key).is_some_and(|value| !value.is_undefined())
}

fn advice(manifest: &JsObject) -> Vec<Finding> {
    let mut found = Vec::new();
    if written(manifest, "capabilities") {
        found.push(warning_at("capabilities", NO_LONGER_READ));
    }
    for key in MOVED_UNDER_EXPERIMENTAL {
        if written(manifest, key) {
            found.push(moved_warning(key));
        }
    }
    found
}

/// The `userConfig` options (and each channel's) a server may reference as `${user_config.*}`.
#[must_use]
pub fn declared_options(manifest: &JsObject) -> IndexSet<String> {
    fn keys(value: Option<&JsValue>) -> Vec<String> {
        match value {
            Some(JsValue::Object(members)) => members.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }
    let channels = manifest
        .get("channels")
        .and_then(JsValue::as_array)
        .map_or(&[][..], Vec::as_slice);
    keys(manifest.get("userConfig"))
        .into_iter()
        .chain(
            channels
                .iter()
                .flat_map(|channel| keys(channel.get("userConfig"))),
        )
        .collect()
}

fn guarded_error(found: &[Finding]) -> bool {
    found.iter().any(|issue| issue.severity == Severity::Error)
}

/// What the inline `hooks` and `mcpServers` hold: hooks as the oracle reads them (no shell advice), servers for references and cleartext URLs.
fn inline_checks(manifest: &JsObject) -> Vec<Finding> {
    let hooks = manifest.get("hooks").unwrap_or(&JsValue::Undefined);
    let maps: Vec<(&JsValue, String)> = hooks.as_array().map_or_else(
        || vec![(hooks, "hooks".to_owned())],
        |entries| {
            entries
                .iter()
                .enumerate()
                .map(|(index, entry)| (entry, format!("hooks.{index}")))
                .collect()
        },
    );
    let from_hooks: Vec<Finding> = maps
        .into_iter()
        .flat_map(|(map, prefix)| {
            map.as_object().map_or_else(Vec::new, |map| {
                hooks_map_findings(
                    map,
                    Some(&HooksSite {
                        prefix,
                        advice: false,
                    }),
                )
            })
        })
        .collect();
    let refused = if guarded_error(&from_hooks) {
        vec![error_at("hooks", "Invalid input")]
    } else {
        Vec::new()
    };
    let servers = match manifest.get("mcpServers") {
        Some(JsValue::Object(servers)) => inline_findings(servers, &declared_options(manifest)),
        _ => Vec::new(),
    };
    from_hooks
        .into_iter()
        .chain(refused)
        .chain(servers)
        .collect()
}

/// `claude plugin validate`'s checks of a parsed `plugin.json`.
#[must_use]
pub fn check_plugin_manifest(raw: &JsValue, options: &ManifestOptions) -> ManifestChecks {
    let JsValue::Object(raw) = raw else {
        return ManifestChecks {
            findings: vec![error_at("root", "manifest must be an object")],
            inline: Vec::new(),
            claims: Vec::new(),
        };
    };
    let claimed = claims_of(raw);
    let schema: Vec<Finding> = missing_name(raw)
        .into_iter()
        .chain(field_findings(raw))
        .chain(experimental_findings(raw.get("experimental")))
        .collect();
    let sound = schema.is_empty();
    let name = match raw.get("name") {
        Some(JsValue::String(name)) => {
            let reported = options.entry_name.as_deref() == Some(name.as_str());
            let name_options = NameOptions {
                official: options.official,
                context: options.context,
            };
            name_findings(name, name_options, reported)
        }
        _ => NameFindings {
            blocking: Vec::new(),
            advice: Vec::new(),
        },
    };
    let recommended = RECOMMENDED
        .iter()
        .filter(|(key, _)| {
            raw.get(*key)
                .is_none_or(|value| value.is_undefined() || value.as_str() == Some(""))
        })
        .map(|(key, message)| warning_at(key, message));
    let findings = name
        .blocking
        .into_iter()
        .chain(schema)
        .chain(claimed.findings)
        .chain(if sound { name.advice } else { Vec::new() })
        .chain(unknown_keys(raw))
        .chain(loose_object_findings(raw))
        .chain(advice(raw))
        .chain(recommended.filter(|_| sound))
        .collect();
    ManifestChecks {
        findings,
        inline: if sound {
            inline_checks(raw)
        } else {
            Vec::new()
        },
        claims: claimed.claims,
    }
}

crate::registry::exports! {
    "format/validate/plugin-manifest" {
        "checkPluginManifest" => ok |raw: JsValue, options: Option<ManifestOptions>|
            check_plugin_manifest(&raw, &options.unwrap_or_default());
    }
}
