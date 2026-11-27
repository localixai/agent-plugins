//! Port of `src/format/manifest.ts`, switched.
//!
//! Reading `.claude-plugin/plugin.json` as Claude Code loads it (plugins/manifest-reference.md):
//! an unrecognized top-level key is stripped with a warning; a key of the wrong type, an unknown key
//! in a strict object, a bad path or an unusable name is an error, and a manifest with an error does
//! not load ("Validation failed … Claude Code reports the same problem when it loads the plugin").

use serde::Serialize;

use crate::format::issues::{PluginIssue, blocks_loading, error, warning};
use crate::format::manifest_schema::{
    MANIFEST_SCHEMA, MANIFEST_SCHEMAS, USER_CONFIG_OPTION_SCHEMA, is_manifest_key,
};
use crate::format::names::name_load_issues;
use crate::format::user_config_refs::USER_CONFIG_KEY;
use crate::format::validate::finding::Finding;
use crate::format::validate::plugin_manifest::{
    ManifestOptions, check_plugin_manifest, manifest_path_findings,
};
use crate::js::{JsObject, JsValue, regex_test, string_of};
use crate::zod::ZodError;

/// A manifest read for loading: `manifest` is `undefined` when an error keeps the plugin from loading.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ManifestResult {
    /// The manifest that loaded, or `undefined`.
    #[serde(skip_serializing_if = "JsValue::is_undefined")]
    pub manifest: JsValue,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// Keys moved under `experimental` that still load at the top level ("Fields").
const MOVED: [&str; 2] = ["themes", "monitors"];

/// Each problem with the path below the key it is at (`go: Unrecognized key: "foo"`).
fn describe(failure: &ZodError) -> String {
    let text = failure
        .issues
        .iter()
        .map(|found| {
            if found.path().is_empty() {
                found.message().to_owned()
            } else {
                let path: Vec<String> = found.path().iter().map(string_of).collect();
                format!("{}: {}", path.join("."), found.message())
            }
        })
        .collect::<Vec<_>>()
        .join("; ");
    if text.is_empty() {
        "invalid value".to_owned()
    } else {
        text
    }
}

/// `userConfig` reported per option, so the issue names the option at fault.
fn user_config_issues(value: &JsValue) -> Vec<PluginIssue> {
    let JsValue::Object(options) = value else {
        return vec![error(
            "userConfig",
            "Invalid \"userConfig\": expected an object",
        )];
    };
    options
        .iter()
        .flat_map(|(key, option)| {
            let path = format!("userConfig.{key}");
            if !regex_test(&USER_CONFIG_KEY, key) {
                let message =
                    format!("Invalid option key \"{key}\": letters, digits and underscores only");
                return vec![error(&path, &message)];
            }
            match USER_CONFIG_OPTION_SCHEMA.safe_parse(option) {
                Ok(_) => Vec::new(),
                Err(failure) => {
                    let message = format!("Invalid option \"{key}\": {}", describe(&failure));
                    vec![error(&path, &message)]
                }
            }
        })
        .collect()
}

fn key_issues(key: &str, value: &JsValue) -> Vec<PluginIssue> {
    let Some(schema) = MANIFEST_SCHEMAS.get(key) else {
        return vec![warning(key, &format!("Unknown field \"{key}\""))];
    };
    if let Err(failure) = schema.safe_parse(value) {
        return if key == "userConfig" {
            user_config_issues(value)
        } else {
            vec![error(
                key,
                &format!("Invalid \"{key}\": {}", describe(&failure)),
            )]
        };
    }
    if MOVED.contains(&key) {
        vec![warning(
            key,
            &format!("\"{key}\" belongs under \"experimental\""),
        )]
    } else {
        Vec::new()
    }
}

/// A finding of the validator as the issue loading reports.
fn issue_of(found: Finding) -> PluginIssue {
    PluginIssue {
        severity: found.severity,
        path: found.path,
        message: found.message,
        guidance: None,
    }
}

/// `plugin.json` read as Claude Code loads it.
#[must_use]
pub fn read_manifest(raw: &JsValue) -> ManifestResult {
    let JsValue::Object(raw) = raw else {
        return ManifestResult {
            manifest: JsValue::Undefined,
            issues: vec![error("plugin.json", "The manifest is not a JSON object")],
        };
    };
    let mut issues = Vec::new();
    let mut kept = JsObject::new();
    for (key, value) in raw {
        let found = key_issues(key, value);
        let blocks = blocks_loading(&found);
        issues.extend(found);
        if is_manifest_key(key) && !blocks {
            kept.insert(key.clone(), value.clone());
        }
    }
    let Some(JsValue::String(name)) = kept.get("name") else {
        if !issues.iter().any(|found| found.path == "name") {
            issues.push(error("name", "The manifest has no \"name\""));
        }
        return ManifestResult {
            manifest: JsValue::Undefined,
            issues,
        };
    };
    issues.extend(name_load_issues(name));
    // Every kept key passed its own schema, so the whole parses.
    let parsed = MANIFEST_SCHEMA.safe_parse(&JsValue::Object(kept.clone()));
    // An error so far, or a whole that does not parse: nothing loads.
    let (false, Ok(manifest)) = (blocks_loading(&issues), parsed) else {
        return ManifestResult {
            manifest: JsValue::Undefined,
            issues,
        };
    };
    issues.extend(manifest_path_findings(&kept).into_iter().map(issue_of));
    ManifestResult {
        manifest: if blocks_loading(&issues) {
            JsValue::Undefined
        } else {
            manifest
        },
        issues,
    }
}

/// `claude plugin validate` on a parsed `plugin.json`: the validator's findings and what the inline
/// `hooks` and `mcpServers` hold (`checkPluginManifest`, the one rule set).
#[must_use]
pub fn validate_manifest(raw: &JsValue) -> Vec<PluginIssue> {
    let checks = check_plugin_manifest(raw, &ManifestOptions::default());
    checks
        .findings
        .into_iter()
        .chain(checks.inline)
        .map(issue_of)
        .collect()
}

crate::registry::exports! {
    "format/manifest" {
        "readManifest" => ok |raw: JsValue| read_manifest(&raw);
        "validateManifest" => ok |raw: JsValue| validate_manifest(&raw);
    }
}
