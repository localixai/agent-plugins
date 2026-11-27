//! Port of `src/convert/to-core/manifest.ts`, switched.
//!
//! The root `plugin.json` of a Claude Code plugin (Agent Plugins Extended §18.4.3 item 1): the name
//! must satisfy core §5.5 or the conversion stops (§18.4.1 item 2: never renamed); the canonical
//! `$schema` and the nine shared fields are written; every other manifest field is reported. The
//! component keys `skills`, `hooks` and `mcpServers` are converted through the components they load
//! and reported there.

use serde::{Deserialize, Serialize};

use crate::convert::notes::{ConversionNote, not_converted, stop};
use crate::convert::to_claude_code::manifest::{CLAUDE_MANIFEST, SHARED_FIELDS};
use crate::format::core::name::core_name_problems;
use crate::format::core::versions::{CoreDocument, CoreVersion, core_schema_id};
use crate::js::{JsObject, JsValue, property_order};

/// Where the converted manifest is written.
pub const CORE_MANIFEST: &str = "plugin.json";

const THROUGH_COMPONENTS: [&str; 3] = ["skills", "hooks", "mcpServers"];

/// What of a loaded `Plugin` the manifest reads: its name and manifest.
#[derive(Clone, Debug, Deserialize)]
pub struct PluginNaming {
    /// The plugin's name.
    pub name: String,
    /// The manifest, when there is one.
    #[serde(default)]
    pub manifest: JsValue,
}

/// The converted manifest and its notes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CoreManifestPart {
    /// What `plugin.json` holds.
    pub value: JsObject,
    /// What the conversion says of it.
    pub notes: Vec<ConversionNote>,
}

/// The root `plugin.json` of a Claude Code plugin, and its notes.
#[must_use]
pub fn core_manifest_of(plugin: &PluginNaming) -> CoreManifestPart {
    let mut fallback = JsObject::new();
    fallback.insert("name".to_owned(), JsValue::String(plugin.name.clone()));
    let manifest = plugin.manifest.as_object().unwrap_or(&fallback);
    let naming = core_name_problems(&plugin.name).into_iter().map(|problem| {
        stop(
            CLAUDE_MANIFEST.to_owned(),
            Some("name".to_owned()),
            format!(
                "the name \"{}\" is not a core package name ({problem}), and a converter never renames",
                plugin.name
            ),
        )
    });
    let others = property_order(manifest)
        .into_iter()
        .map(|(key, _)| key)
        .filter(|key| {
            !SHARED_FIELDS.contains(&key.as_str()) && !THROUGH_COMPONENTS.contains(&key.as_str())
        })
        .map(|key| {
            not_converted(
                CLAUDE_MANIFEST.to_owned(),
                Some(key.clone()),
                format!("\"{key}\" has no core counterpart"),
            )
        });
    let mut value = JsObject::new();
    value.insert(
        "$schema".to_owned(),
        JsValue::String(core_schema_id(CoreVersion::V1_1_0, CoreDocument::Plugin)),
    );
    value.insert("name".to_owned(), JsValue::String(plugin.name.clone()));
    for field in SHARED_FIELDS {
        if let Some(member) = manifest.get(field).filter(|member| !member.is_undefined())
            && field != "name"
        {
            value.insert(field.to_owned(), member.clone());
        }
    }
    CoreManifestPart {
        value,
        notes: naming.chain(others).collect(),
    }
}

crate::registry::exports! {
    "convert/to-core/manifest" {
        "CORE_MANIFEST" => ok || CORE_MANIFEST;
        "coreManifestOf" => ok |plugin: PluginNaming| core_manifest_of(&plugin);
    }
}
