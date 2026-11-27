//! Port of `src/marketplace/installed-plugins.ts`, switched.
//!
//! `installed_plugins.json`, the record of what Claude Code has installed (plugins/loading.md,
//! "Check which stage a plugin reached": "records each install with its `scope`, `installPath`,
//! and `version`"; "Find plugins on disk"), and what it does with a file or a record it cannot use
//! (plugins/troubleshooting.md: "could not be read and was rebuilt", "records under names that no
//! version of Claude Code can use were removed", "holds a record under `"<id>"` that this version
//! of Claude Code cannot read"). The Agent SDK types no such file, so the shapes are declared here
//! from the docs. Pure: reading and writing the file and its dated copies is `PluginStateStore`'s.
//!
//! Judgments (spec/judgments.md): the file is `{ "version": 2, "plugins": { "<id>": [record, ...]
//! } }` (the docs name a "format (version `<N>`)" and `projectPath`, not the whole layout);
//! `installedAt`, `lastUpdated`, `gitCommitSha` and `projectPath` are optional text; fields the kit
//! does not know are kept and written back; a file with no numeric `version` is not a list of
//! plugins in any format and is rebuilt.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::install_messages::{unknown_format_message, unreadable_records_message};
use super::install_scopes::{INSTALL_SCOPES, InstallScope};
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::ids::is_plugin_id_part;
use crate::js::json_parse::json_parse;
use crate::js::{JsObject, JsValue, in_property_order, json_stringify_indented, property_order};
use crate::protocol::is_record::is_record;

/// The format this version of the kit reads and writes.
const INSTALLED_PLUGINS_VERSION: f64 = 2.0;

/// One install of a plugin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRecord {
    /// The scope it is installed in.
    pub scope: InstallScope,
    /// Where the installed copy is; for a copied plugin `cache/<marketplace>/<plugin>/<version>/`.
    pub install_path: String,
    /// Its version.
    pub version: String,
    /// When it was installed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<String>,
    /// When it was last updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    /// The commit a git-sourced plugin was installed at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_sha: Option<String>,
    /// The project folder of a `project` or `local` install.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Fields this version does not know, kept so a rewrite loses nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<JsObject>,
}

/// Plugin id (`name@marketplace`) to its installs, one per scope (and project).
pub type InstalledRecords = IndexMap<String, Vec<InstallRecord>>;

/// A file that is not a list of plugins: its text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Rebuilt {
    /// The text, to keep, before the file is rebuilt empty.
    pub text: String,
}

/// What a file's text says.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPluginsRead {
    /// The records this version can use.
    pub records: InstalledRecords,
    /// Raw values under keys that are not plugin ids: copy them aside, then drop them.
    pub set_aside: JsObject,
    /// Ids whose value this version cannot read, in file order.
    pub unreadable: Vec<String>,
    /// Whether the file may be rewritten: never while a record or the format cannot be read.
    pub writable: bool,
    /// What is wrong.
    pub issues: Vec<PluginIssue>,
    /// The `version` of a file whose format this version does not know.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown_format: Option<f64>,
    /// A file that is not a list of plugins: its text, to keep, before the file is rebuilt empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rebuilt: Option<Rebuilt>,
}

fn nothing() -> InstalledPluginsRead {
    InstalledPluginsRead {
        records: IndexMap::new(),
        set_aside: JsObject::new(),
        unreadable: Vec::new(),
        writable: true,
        issues: Vec::new(),
        unknown_format: None,
        rebuilt: None,
    }
}

const OPTIONAL_TEXT: [&str; 4] = ["installedAt", "lastUpdated", "gitCommitSha", "projectPath"];
const KNOWN_KEYS: [&str; 7] = [
    "scope",
    "installPath",
    "version",
    "installedAt",
    "lastUpdated",
    "gitCommitSha",
    "projectPath",
];

fn scope_of(value: Option<&JsValue>) -> Option<InstallScope> {
    let text = value?.as_str()?;
    INSTALL_SCOPES
        .into_iter()
        .find(|scope| scope.as_str() == text)
}

/// The optional text fields, in `OPTIONAL_TEXT`'s order; `None` when one is not text.
fn optional_text(raw: &JsObject) -> Option<[Option<String>; 4]> {
    let mut found: [Option<String>; 4] = Default::default();
    for (slot, key) in found.iter_mut().zip(OPTIONAL_TEXT) {
        match raw.get(key) {
            None | Some(JsValue::Undefined) => {}
            Some(JsValue::String(value)) => *slot = Some(value.clone()),
            Some(_) => return None,
        }
    }
    Some(found)
}

fn read_record(raw: &JsValue) -> Option<InstallRecord> {
    let members = raw.as_object().filter(|_| is_record(raw))?;
    let scope = scope_of(members.get("scope"));
    let optional = optional_text(members);
    let (scope, optional) = (scope?, optional?);
    let install_path = members.get("installPath").and_then(JsValue::as_str)?;
    let version = members.get("version").and_then(JsValue::as_str)?;
    if install_path.is_empty() {
        return None;
    }
    let extra: JsObject = property_order(members)
        .into_iter()
        .filter(|(key, _)| !KNOWN_KEYS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let [installed_at, last_updated, git_commit_sha, project_path] = optional;
    Some(InstallRecord {
        scope,
        install_path: install_path.to_owned(),
        version: version.to_owned(),
        installed_at,
        last_updated,
        git_commit_sha,
        project_path,
        extra: (!extra.is_empty()).then_some(extra),
    })
}

/// A list of records, every one of which must read; anything else is not this version's.
fn read_records(value: &JsValue) -> Option<Vec<InstallRecord>> {
    value.as_array()?.iter().map(read_record).collect()
}

/// A plugin id is `<name>@<origin>`: one `@`, each side a part of a plugin id.
fn is_usable_id(key: &str) -> bool {
    let parts: Vec<&str> = key.split('@').collect();
    parts.len() == 2 && parts.iter().all(|part| is_plugin_id_part(part))
}

fn read_plugins(plugins: &JsObject) -> InstalledPluginsRead {
    let mut read = nothing();
    for (key, value) in property_order(plugins) {
        if !is_usable_id(key) {
            read.set_aside.insert(key.clone(), value.clone());
            continue;
        }
        match read_records(value) {
            Some(records) => {
                read.records.insert(key.clone(), records);
            }
            None => read.unreadable.push(key.clone()),
        }
    }
    read.writable = read.unreadable.is_empty();
    if !read.unreadable.is_empty() {
        read.issues = vec![error(
            "installed_plugins.json",
            &unreadable_records_message(&read.unreadable),
        )];
    }
    read
}

/// What a file's text says. A file that "isn't valid JSON, or isn't a list of plugins" is rebuilt
/// (its text kept); a key that is not a plugin id has its record set aside; a record whose fields
/// do not parse is named and keeps the file from being rewritten; a format this version does not
/// know is named and read no further. No file is no installs.
#[must_use]
pub fn parse_installed_plugins(text: Option<&str>) -> InstalledPluginsRead {
    let Some(text) = text else {
        return nothing();
    };
    let parsed = json_parse(text).unwrap_or(JsValue::Undefined);
    let version = parsed.get("version").and_then(JsValue::as_f64);
    let plugins = parsed
        .get("plugins")
        .filter(|plugins| is_record(plugins))
        .and_then(JsValue::as_object);
    let (Some(version), Some(plugins)) = (version, plugins) else {
        return InstalledPluginsRead {
            rebuilt: Some(Rebuilt {
                text: text.to_owned(),
            }),
            ..nothing()
        };
    };
    if version != INSTALLED_PLUGINS_VERSION {
        return InstalledPluginsRead {
            unknown_format: Some(version),
            writable: false,
            issues: vec![error(
                "installed_plugins.json",
                &unknown_format_message(version),
            )],
            ..nothing()
        };
    }
    read_plugins(plugins)
}

/// A record as the file holds it: `{ ...extra, ...fields }`, the fields in the order the record
/// holds them.
fn wire(record: &JsValue) -> JsValue {
    let Some(members) = record.as_object() else {
        return record.clone();
    };
    let mut out = JsObject::new();
    if let Some(extra) = members.get("extra").and_then(JsValue::as_object) {
        for (key, value) in property_order(extra) {
            out.insert(key.clone(), value.clone());
        }
    }
    for (key, value) in property_order(members) {
        if key != "extra" {
            out.insert(key.clone(), value.clone());
        }
    }
    JsValue::Object(in_property_order(out))
}

/// The file's text for these records: two-space indented JSON and a final newline (a judgment).
/// `records` is an object of arrays of records, as the host holds it, so its members keep their
/// order.
#[must_use]
pub fn serialize_installed_plugins(records: &JsObject) -> String {
    let plugins: JsObject = property_order(records)
        .into_iter()
        .map(|(id, installs)| {
            let wired = installs
                .as_array()
                .map_or_else(Vec::new, |installs| installs.iter().map(wire).collect());
            (id.clone(), JsValue::Array(wired))
        })
        .collect();
    let file: JsObject = [
        (
            "version".to_owned(),
            JsValue::Number(INSTALLED_PLUGINS_VERSION),
        ),
        ("plugins".to_owned(), JsValue::Object(plugins)),
    ]
    .into_iter()
    .collect();
    let text = json_stringify_indented(&JsValue::Object(file), 2).unwrap_or_default();
    format!("{text}\n")
}

/// Typed records as the object `serialize_installed_plugins` reads, for the core's own callers.
#[must_use]
pub fn records_value(records: &InstalledRecords) -> JsObject {
    match crate::js::value::to_js_value(records) {
        Ok(JsValue::Object(members)) => members,
        _ => JsObject::new(),
    }
}

/// The records as the host passed them.
#[derive(Deserialize)]
#[serde(transparent)]
struct GivenRecords(JsObject);

crate::registry::exports! {
    "marketplace/installed-plugins" {
        "parseInstalledPlugins" => ok |text: Option<String>| parse_installed_plugins(text.as_deref());
        "serializeInstalledPlugins" => ok |records: GivenRecords| serialize_installed_plugins(&records.0);
    }
}
