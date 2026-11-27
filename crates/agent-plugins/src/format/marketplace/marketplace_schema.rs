//! Port of `src/format/marketplace/marketplace-schema.ts`, switched.
//!
//! The top level of `marketplace.json` (plugins/marketplace-reference.md, "Top-level fields" and
//! "Validation messages"): every key Claude Code reads, the type of each, and the report for a key
//! that is missing, of the wrong type or unknown. `name` and `owner.name` are checked for their
//! grammar by the caller; `plugins` is handed on entry by entry.

use indexmap::IndexMap;
use serde::Serialize;

use super::renames::Renames;
use super::types::{MarketplaceMetadata, Owner};
use crate::format::issues::{PluginIssue, Severity, error, warning};
use crate::format::validate::received::kind_of;
use crate::format::validate::unknown_fields::{FieldSite, unknown_field_finding};
use crate::js::{JsObject, JsValue, property_order};

/// Every key of a `marketplace.json` Claude Code reads.
pub const MARKETPLACE_KEYS: [&str; 10] = [
    "name",
    "owner",
    "plugins",
    "$schema",
    "description",
    "version",
    "metadata",
    "forceRemoveDeletedPlugins",
    "allowCrossMarketplaceDependenciesOn",
    "renames",
];

/// Every key of `metadata` Claude Code reads.
pub const METADATA_KEYS: [&str; 3] = ["description", "version", "pluginRoot"];

fn is_marketplace_key(key: &str) -> bool {
    MARKETPLACE_KEYS.contains(&key)
}
fn is_metadata_key(key: &str) -> bool {
    METADATA_KEYS.contains(&key)
}

/// The JSON type of a value, as the `got <type>` and `received <type>` reports name it.
#[must_use]
pub const fn type_name(value: &JsValue) -> &'static str {
    kind_of(value)
}

/// A required field that is absent. The docs quote `owner: Invalid input` as an example; the
/// oracle (docs/SPEC.md) names the type that was expected after it, and so does the kit.
fn missing(path: &str, expected: &str) -> PluginIssue {
    error(
        path,
        &format!("Invalid input: expected {expected}, received undefined"),
    )
}

/// `Invalid input: expected <type>, received <type>`, the one wording of a value of the wrong
/// type.
#[must_use]
pub fn mismatch(path: &str, expected: &str, value: &JsValue) -> PluginIssue {
    error(
        path,
        &format!(
            "Invalid input: expected {expected}, received {}",
            type_name(value)
        ),
    )
}

/// What a reader returns: the value when it is usable, and the issues it raised.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Read<T> {
    /// The value, when it is usable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    /// What it raised.
    pub issues: Vec<PluginIssue>,
}

const fn ok<T>(value: T) -> Read<T> {
    Read {
        value: Some(value),
        issues: Vec::new(),
    }
}
const fn absent<T>() -> Read<T> {
    Read {
        value: None,
        issues: Vec::new(),
    }
}
fn fail<T>(issues: Vec<PluginIssue>) -> Read<T> {
    Read {
        value: None,
        issues,
    }
}

fn optional<T>(
    value: &JsValue,
    path: &str,
    expected: &str,
    guard: impl Fn(&JsValue) -> Option<T>,
) -> Read<T> {
    if value.is_undefined() {
        return absent();
    }
    guard(value).map_or_else(|| fail(vec![mismatch(path, expected, value)]), ok)
}

fn is_string(value: &JsValue) -> Option<String> {
    value.as_str().map(str::to_owned)
}
fn is_boolean(value: &JsValue) -> Option<bool> {
    value.as_bool()
}

fn optional_string(value: &JsValue, path: &str) -> Read<String> {
    optional(value, path, "string", is_string)
}

/// Reads the optional string keys of `record` named in `keys`, each at `<prefix><key>`.
fn string_fields(
    record: &JsObject,
    keys: &[&'static str],
    prefix: &str,
) -> (IndexMap<&'static str, String>, Vec<PluginIssue>) {
    let mut values = IndexMap::new();
    let mut issues = Vec::new();
    for key in keys {
        let read = optional_string(
            record.get(*key).unwrap_or(&JsValue::Undefined),
            &format!("{prefix}{key}"),
        );
        issues.extend(read.issues);
        if let Some(value) = read.value {
            values.insert(*key, value);
        }
    }
    (values, issues)
}

const OWNER_STRINGS: [&str; 2] = ["email", "url"];

/// `owner`: an object with a `name` (non-empty) and optional `email` and `url`.
#[must_use]
pub fn read_owner(value: &JsValue) -> Read<Owner> {
    if value.is_undefined() {
        return fail(vec![missing("owner", "object")]);
    }
    let JsValue::Object(members) = value else {
        return fail(vec![mismatch("owner", "object", value)]);
    };
    let name = members.get("name").unwrap_or(&JsValue::Undefined);
    if name.is_undefined() {
        return fail(vec![missing("owner.name", "string")]);
    }
    let Some(name) = name.as_str() else {
        return fail(vec![mismatch("owner.name", "string", name)]);
    };
    if name.is_empty() {
        return fail(vec![error("owner.name", "Author name cannot be empty")]);
    }
    let (mut rest, issues) = string_fields(members, &OWNER_STRINGS, "owner.");
    if !issues.is_empty() {
        return fail(issues);
    }
    ok(Owner {
        name: name.to_owned(),
        email: rest.shift_remove("email"),
        url: rest.shift_remove("url"),
    })
}

/// `metadata`: `description`, `version` and `pluginRoot`; any other key warns and is kept under
/// `extra`.
fn read_metadata(value: &JsValue) -> Read<MarketplaceMetadata> {
    if value.is_undefined() {
        return absent();
    }
    let JsValue::Object(members) = value else {
        return fail(vec![mismatch("metadata", "object", value)]);
    };
    let extra_keys: Vec<&String> = property_order(members)
        .into_iter()
        .map(|(key, _)| key)
        .filter(|key| !is_metadata_key(key))
        .collect();
    let unknown: Vec<PluginIssue> = extra_keys
        .iter()
        .map(|key| unknown_field(key, &format!("metadata.{key}"), &METADATA_KEYS))
        .collect();
    let (mut strings, issues) = string_fields(members, &METADATA_KEYS, "metadata.");
    if !issues.is_empty() {
        return fail(issues.into_iter().chain(unknown).collect());
    }
    let extra: IndexMap<String, JsValue> = extra_keys
        .iter()
        .map(|key| {
            (
                (*key).clone(),
                members.get(*key).cloned().unwrap_or(JsValue::Undefined),
            )
        })
        .collect();
    Read {
        value: Some(MarketplaceMetadata {
            description: strings.shift_remove("description"),
            version: strings.shift_remove("version"),
            plugin_root: strings.shift_remove("pluginRoot"),
            extra: (!extra_keys.is_empty()).then_some(extra),
        }),
        issues: unknown,
    }
}

/// The warning for a key Claude Code does not read, at `path`: the docs quote its plain form
/// (`Unknown field '<key>'`); the oracle adds where the key probably comes from (docs/SPEC.md).
#[must_use]
pub fn unknown_field(key: &str, path: &str, known: &[&str]) -> PluginIssue {
    let known: Vec<String> = known.iter().map(|known| (*known).to_owned()).collect();
    let found = unknown_field_finding(key, &known, path, FieldSite::Other);
    warning(&found.path, &found.message)
}

fn read_string_list(value: &JsValue, path: &str) -> Read<Vec<String>> {
    if value.is_undefined() {
        return absent();
    }
    let Some(items) = value.as_array() else {
        return fail(vec![mismatch(path, "array", value)]);
    };
    if let Some(bad) = items.iter().position(|item| !item.is_string()) {
        return fail(vec![mismatch(
            &format!("{path}[{bad}]"),
            "string",
            items.get(bad).unwrap_or(&JsValue::Undefined),
        )]);
    }
    ok(items.iter().filter_map(is_string).collect())
}

/// `renames`: old plugin name to the new one, or `null` for a plugin that was removed.
fn read_renames(value: &JsValue) -> Read<Renames> {
    if value.is_undefined() {
        return absent();
    }
    let JsValue::Object(members) = value else {
        return fail(vec![mismatch("renames", "object", value)]);
    };
    let mut entries = Renames::new();
    for (old, target) in property_order(members) {
        match target {
            JsValue::Null => entries.insert(old.clone(), None),
            JsValue::String(text) => entries.insert(old.clone(), Some(text.clone())),
            JsValue::Undefined
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::Array(_)
            | JsValue::Object(_) => {
                return fail(vec![mismatch(
                    &format!("renames.{old}"),
                    "string or null",
                    target,
                )]);
            }
        };
    }
    ok(entries)
}

/// `name`: required; empty is "Marketplace must have a name"; the grammar is the caller's.
fn read_name(value: &JsValue) -> Read<String> {
    if value.is_undefined() {
        return fail(vec![missing("name", "string")]);
    }
    let Some(name) = value.as_str() else {
        return fail(vec![mismatch("name", "string", value)]);
    };
    if name.is_empty() {
        return fail(vec![error("name", "Marketplace must have a name")]);
    }
    ok(name.to_owned())
}

/// `plugins`: required array, handed on as it is.
fn read_plugins(value: &JsValue) -> Read<Vec<JsValue>> {
    if value.is_undefined() {
        return fail(vec![missing("plugins", "array")]);
    }
    value.as_array().map_or_else(
        || fail(vec![mismatch("plugins", "array", value)]),
        |items| ok(items.clone()),
    )
}

/// The top-level fields of a marketplace, everything but its entries.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopLevel {
    /// Its name.
    pub name: String,
    /// Its owner.
    pub owner: Owner,
    /// Its entries, as written.
    pub plugins: Vec<JsValue>,
    /// What it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Its metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MarketplaceMetadata>,
    /// Whether a plugin that left the list is removed.
    pub force_remove_deleted_plugins: bool,
    /// The marketplaces a plugin may depend on.
    pub allow_cross_marketplace_dependencies_on: Vec<String>,
    /// Old plugin name to the new one, or `null` for a plugin that was removed.
    pub renames: Renames,
}

/// What reading the top level gave.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TopLevelResult {
    /// Undefined when any field breaks the schema: such a marketplace does not load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<TopLevel>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// Reads and checks every top-level key of the parsed `marketplace.json`.
#[must_use]
pub fn read_top_level(raw: &JsObject) -> TopLevelResult {
    let field = |key: &str| raw.get(key).unwrap_or(&JsValue::Undefined);
    let name = read_name(field("name"));
    let owner = read_owner(field("owner"));
    let plugins = read_plugins(field("plugins"));
    let schema = optional_string(field("$schema"), "$schema");
    let description = optional_string(field("description"), "description");
    let version = optional_string(field("version"), "version");
    let metadata = read_metadata(field("metadata"));
    let force = optional(
        field("forceRemoveDeletedPlugins"),
        "forceRemoveDeletedPlugins",
        "boolean",
        is_boolean,
    );
    let allow = read_string_list(
        field("allowCrossMarketplaceDependenciesOn"),
        "allowCrossMarketplaceDependenciesOn",
    );
    let renames = read_renames(field("renames"));
    let unknown = property_order(raw)
        .into_iter()
        .map(|(key, _)| key)
        .filter(|key| !is_marketplace_key(key))
        .map(|key| unknown_field(key, key, &MARKETPLACE_KEYS));
    let mut issues: Vec<PluginIssue> = Vec::new();
    issues.extend(name.issues);
    issues.extend(owner.issues);
    issues.extend(plugins.issues);
    issues.extend(schema.issues);
    issues.extend(description.issues.clone());
    issues.extend(version.issues.clone());
    issues.extend(metadata.issues.clone());
    issues.extend(force.issues);
    issues.extend(allow.issues.clone());
    issues.extend(renames.issues.clone());
    issues.extend(unknown);
    let (Some(name), Some(owner), Some(plugins)) = (name.value, owner.value, plugins.value) else {
        return TopLevelResult { top: None, issues };
    };
    if issues.iter().any(|found| found.severity == Severity::Error) {
        return TopLevelResult { top: None, issues };
    }
    TopLevelResult {
        top: Some(TopLevel {
            name,
            owner,
            plugins,
            description: description.value,
            version: version.value,
            metadata: metadata.value,
            force_remove_deleted_plugins: force.value.unwrap_or(false),
            allow_cross_marketplace_dependencies_on: allow.value.unwrap_or_default(),
            renames: renames.value.unwrap_or_default(),
        }),
        issues,
    }
}

crate::registry::exports! {
    "format/marketplace/marketplace-schema" {
        "mismatch" => ok |path: String, expected: String, value: JsValue| mismatch(&path, &expected, &value);
        "readTopLevel" => ok |raw: JsObject| read_top_level(&raw);
    }
}
