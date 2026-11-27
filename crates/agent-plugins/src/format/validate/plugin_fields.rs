//! Ported from `src/format/validate/plugin-fields.ts`; only the core uses it now, so it has no
//! wrapper.
//!
//! The type rules of `plugin.json` as `claude plugin validate` applies them (oracle 2.1.319): one
//! check per key, in the order the validator reports them. A key with no entry here is either a
//! path key (`plugin-paths.ts`), a listing field the validator reads no further, or unknown.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::Deserialize;

use super::finding::{Finding, error_at, warning_at};
use super::inline_sources::source_findings;
use super::mcp_check::inline_servers_valid;
use super::plugin_paths::{PathKey, is_path_key};
use super::received::kind_of;
use super::zod_findings::zod_findings;
use crate::format::manifest_schema::{DEPENDENCY_TEXT, USER_CONFIG_OPTION_SCHEMA};
use crate::format::user_config_refs::USER_CONFIG_KEY_SOURCE as USER_CONFIG_KEY;
use crate::js::{JsObject, JsValue};
use crate::zod::{self, Schema, fields};

static TEXT: LazyLock<Schema> = LazyLock::new(zod::string);
static FLAG: LazyLock<Schema> = LazyLock::new(zod::boolean);

/// One option rule set for loading and the validator (manifest-reference.md, "User configuration").
static USER_CONFIG: LazyLock<Schema> = LazyLock::new(|| {
    zod::record(
        zod::string().regex(USER_CONFIG_KEY),
        USER_CONFIG_OPTION_SCHEMA.clone(),
    )
});

static SCHEMAS: LazyLock<IndexMap<&'static str, Schema>> = LazyLock::new(|| {
    IndexMap::from([
        ("name", TEXT.clone()),
        ("displayName", TEXT.clone()),
        ("$schema", TEXT.clone()),
        ("version", TEXT.clone()),
        ("description", TEXT.clone()),
        (
            "author",
            zod::object(
                fields! { "name" => TEXT.clone(), "email" => TEXT.optional(), "url" => TEXT.optional() },
            ),
        ),
        ("homepage", zod::url()),
        ("repository", TEXT.clone()),
        ("license", TEXT.clone()),
        ("keywords", zod::array(TEXT.clone())),
        ("defaultEnabled", FLAG.clone()),
        (
            "dependencies",
            zod::array(zod::union(vec![
                DEPENDENCY_TEXT.clone(),
                zod::loose_object(fields! { "name" => TEXT.clone() }),
            ])),
        ),
        (
            "channels",
            zod::array(zod::object(fields! {
                "server" => TEXT.clone(),
                "displayName" => TEXT.optional(),
                "userConfig" => USER_CONFIG.optional(),
            })),
        ),
        ("settings", zod::record(TEXT.clone(), zod::unknown())),
        ("userConfig", USER_CONFIG.clone()),
    ])
});

/// The order the oracle reports keys in: the schema's own.
const ORDER: [&str; 26] = [
    "name",
    "displayName",
    "$schema",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "defaultEnabled",
    "dependencies",
    "hooks",
    "commands",
    "agents",
    "skills",
    "outputStyles",
    "workflows",
    "themes",
    "monitors",
    "types",
    "channels",
    "mcpServers",
    "lspServers",
    "settings",
    "userConfig",
];

/// Keys the validator accepts without reading their value.
const UNREAD: [&str; 13] = [
    "metadata",
    "experimental",
    "capabilities",
    "icon",
    "documentationUrl",
    "supportUrl",
    "privacyPolicyUrl",
    "termsOfServiceUrl",
    "bugs",
    // `support` and `privacy_policy` are not here: they draw the "manifests across many tools" warning (J2.7).
    "privacyPolicy",
    "terms_of_service",
    "screenshots",
    "docs",
];

/// Every key a manifest may carry without an unknown-field warning.
#[must_use]
pub fn plugin_keys() -> Vec<&'static str> {
    ORDER.iter().chain(UNREAD.iter()).copied().collect()
}

/// The keys a misspelt one is compared with: those the validator reads, not those it only lets through.
pub const SUGGESTED_KEYS: [&str; 26] = ORDER;

fn key_findings(key: &str, value: &JsValue) -> Vec<Finding> {
    if let Some(schema) = SCHEMAS.get(key) {
        return zod_findings(&schema.safe_parse(value), key, "");
    }
    if let JsValue::Object(servers) = value
        && key == "mcpServers"
        && !inline_servers_valid(servers)
    {
        return vec![error_at("mcpServers", "Invalid input")];
    }
    if !is_path_key(key) {
        return Vec::new();
    }
    PathKey::deserialize(JsValue::from(key))
        .map(|path_key| source_findings(path_key, key, value))
        .unwrap_or_default()
}

/// The type errors of the keys present, in the validator's order.
#[must_use]
pub fn field_findings(manifest: &JsObject) -> Vec<Finding> {
    ORDER
        .iter()
        .flat_map(|key| match manifest.get(*key) {
            Some(value) if !value.is_undefined() => key_findings(key, value),
            Some(_) | None => Vec::new(),
        })
        .collect()
}

/// `name` is the one key a manifest cannot do without.
#[must_use]
pub fn missing_name(manifest: &JsObject) -> Vec<Finding> {
    if manifest.contains_key("name") {
        Vec::new()
    } else {
        vec![error_at(
            "name",
            "Invalid input: expected string, received undefined",
        )]
    }
}

fn not_an_object(key: &str, what: &str, outcome: &str, value: &JsValue) -> Finding {
    warning_at(
        key,
        &format!(
            "'{key}' must be {what}; got {}. It will be {outcome} at load time.",
            kind_of(value)
        ),
    )
}

/// The free-form `metadata` and the `experimental` block: a wrong type is only a warning.
#[must_use]
pub fn loose_object_findings(manifest: &JsObject) -> Vec<Finding> {
    let mut found = Vec::new();
    let present = |key: &str| manifest.get(key).filter(|value| !value.is_undefined());
    if let Some(metadata) = present("metadata").filter(|value| !value.is_object()) {
        found.push(not_an_object(
            "metadata",
            "a free-form object",
            "ignored",
            metadata,
        ));
    }
    if let Some(experimental) = present("experimental").filter(|value| !value.is_object()) {
        found.push(not_an_object(
            "experimental",
            "an object containing component declarations",
            "ignored",
            experimental,
        ));
    }
    found
}
