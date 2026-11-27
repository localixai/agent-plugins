//! Port of `src/format/marketplace/entry-schema.ts`, switched.
//!
//! The keys of a plugin entry and the type of each (plugins/marketplace-reference.md, "Plugin
//! entries"): the entry's own fields, and every `plugin.json` field ("an entry also accepts every
//! `plugin.json` field") except the directory listing fields, which "claude plugin validate reports
//! ... as an unknown field" (plugins/manifest-reference.md, "Directory listing fields"). The shared
//! fields reuse the manifest's schemas, so an entry and a plugin.json cannot disagree on a type.

use std::sync::LazyLock;

use serde::Serialize;

use super::schema_issues::zod_issues;
use crate::format::issues::PluginIssue;
use crate::format::manifest_schema::MANIFEST_SCHEMAS;
use crate::js::{JsObject, JsValue};
use crate::zod::{self, Schema, Shape, fields};

/// The manifest's keys whose schemas the fields an entry shares reuse: every key but the directory
/// listing fields (`icon`, `documentationUrl`, `supportUrl`, `privacyPolicyUrl`,
/// `termsOfServiceUrl`, unknown keys in an entry) and the keys an entry reads with a reader of its
/// own (`name`, `metadata`, `experimental`, `entries.rs`). A test holds each to `MANIFEST_SCHEMAS`.
const SHARED: [&str; 25] = [
    "$schema",
    "displayName",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "defaultEnabled",
    "dependencies",
    "settings",
    "userConfig",
    "types",
    "channels",
    "skills",
    "commands",
    "agents",
    "hooks",
    "mcpServers",
    "lspServers",
    "outputStyles",
    "workflows",
    "themes",
    "monitors",
];

/// The entry's own fields (the field table of "Plugin entries"), beyond `source`, `relevance` and `metadata`.
fn own() -> Shape {
    fields! {
        "name" => zod::string(),
        "category" => zod::string(),
        "tags" => zod::array(zod::string()),
        "strict" => zod::boolean(),
        "headers" => zod::record(zod::string(), zod::string()),
        "headersHelper" => zod::string(),
    }
}

/// `{ ...SHARED, ...OWN }`.
static FIELD_SCHEMAS: LazyLock<Shape> = LazyLock::new(|| {
    let mut schemas: Shape = SHARED
        .iter()
        .filter_map(|key| {
            MANIFEST_SCHEMAS
                .get(*key)
                .map(|schema| ((*key).to_owned(), schema.clone()))
        })
        .collect();
    schemas.extend(own());
    schemas
});

static FIELDS_SCHEMA: LazyLock<Schema> =
    LazyLock::new(|| zod::object(FIELD_SCHEMAS.clone()).partial());

/// Every key an entry knows: any other is reported as an unknown field.
pub static ENTRY_KEYS: LazyLock<Vec<String>> = LazyLock::new(|| {
    ["source", "relevance", "metadata", "experimental"]
        .into_iter()
        .map(str::to_owned)
        .chain(FIELD_SCHEMAS.keys().cloned())
        .collect()
});

/// Whether `key` is one of `ENTRY_KEYS`.
#[must_use]
pub fn is_entry_key(key: &str) -> bool {
    ENTRY_KEYS.iter().any(|known| known == key)
}

/// `EntryFields`: the fields of an entry that read with a plain schema, as parsed.
pub type EntryFields = JsObject;

/// What reading the plain fields gave.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldsResult {
    /// Undefined when a field breaks its schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<EntryFields>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// Reads the plain fields of a raw entry one by one, so each problem names its own field.
#[must_use]
pub fn read_entry_fields(raw: &JsObject, base: &str) -> FieldsResult {
    let mut kept = JsObject::new();
    let mut issues: Vec<PluginIssue> = Vec::new();
    for (key, schema) in FIELD_SCHEMAS.iter() {
        let value = raw.get(key).unwrap_or(&JsValue::Undefined);
        if value.is_undefined() {
            continue;
        }
        match schema.safe_parse(value) {
            Ok(parsed) => {
                kept.insert(key.clone(), parsed);
            }
            Err(failure) => issues.extend(zod_issues(&failure, &format!("{base}.{key}"))),
        }
    }
    // Every kept key passed its own schema, so the whole parses; this types it without a cast.
    let whole = FIELDS_SCHEMA.safe_parse(&JsValue::Object(kept));
    FieldsResult {
        fields: whole.ok().and_then(|whole| whole.as_object().cloned()),
        issues,
    }
}

crate::registry::exports! {
    "format/marketplace/entry-schema" {
        "ENTRY_KEYS" => ok || ENTRY_KEYS.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::{ENTRY_KEYS, SHARED};
    use crate::format::manifest_schema::MANIFEST_SCHEMAS;

    #[test]
    fn every_shared_key_is_a_manifest_key() {
        for key in SHARED {
            assert!(MANIFEST_SCHEMAS.contains_key(key), "{key}");
        }
        assert_eq!(ENTRY_KEYS.len(), 4 + SHARED.len() + 6);
    }
}
