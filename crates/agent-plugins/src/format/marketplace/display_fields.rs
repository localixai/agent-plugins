//! Port of `src/format/marketplace/display-fields.ts`, switched.
//!
//! The display fields users see in plugin listings and details (plugins/marketplace-reference.md,
//! "Display fields"): `displayName`, `description`, `author`, `homepage`, `repository`, `license` and
//! `keywords`. "For a field you set on the entry, users see the entry's value, even when `plugin.json`
//! sets a different one. For a field the entry leaves unset, users see the `plugin.json` value."
//! Pure over a parsed entry and, when the host can read it, the plugin's `plugin.json`.
//!
//! A `MarketplaceEntry` and a `PluginManifest` are `JsValue`s until their modules are in the core.

use serde::{Deserialize, Serialize};

use crate::format::defined::defined;
use crate::js::{JsObject, JsValue, in_property_order};

/// The display fields of one plugin: `Partial<Pick<PluginManifest, 'displayName' | …>>`.
pub type DisplayFields = JsObject;

/// Before install: "Claude Code can read plugin.json only for entries with a relative-path source".
const READABLE_BEFORE_INSTALL: [(&str, bool); 7] = [
    ("relative", true),
    ("github", false),
    ("url", false),
    ("git-subdir", false),
    ("npm", false),
    ("archive", false),
    ("command", false),
];

fn entry_fields(entry: &JsValue) -> DisplayFields {
    let member = |key: &str| entry.get(key).cloned().unwrap_or_default();
    let mut fields = JsObject::new();
    fields.insert("displayName".to_owned(), member("displayName"));
    fields.insert("description".to_owned(), member("description"));
    if let Some(JsValue::Object(display)) = entry.get("display") {
        for (key, value) in display {
            fields.insert(key.clone(), value.clone());
        }
    }
    in_property_order(defined(fields))
}

fn manifest_fields(manifest: &JsValue) -> DisplayFields {
    let fields: JsObject = [
        "displayName",
        "description",
        "author",
        "homepage",
        "repository",
        "license",
        "keywords",
    ]
    .into_iter()
    .map(|key| {
        (
            key.to_owned(),
            manifest.get(key).cloned().unwrap_or_default(),
        )
    })
    .collect();
    defined(fields)
}

/// Whether the listing is of a plugin not installed yet (`listed`) or one whose files are on disk (`installed`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListingStage {
    /// Not installed yet.
    Listed,
    /// Its files are on disk.
    Installed,
}

/// The display fields of one plugin as users see them at `stage`. `manifest` is the plugin's own
/// `plugin.json` (`undefined` when it has none or the host has not read it); before install it is
/// used only for a relative-path entry, whose files are inside the marketplace.
#[must_use]
pub fn listing_fields(entry: &JsValue, manifest: &JsValue, stage: ListingStage) -> DisplayFields {
    let kind = entry.get("source").and_then(|source| source.get("kind"));
    let readable = stage == ListingStage::Installed
        || READABLE_BEFORE_INSTALL
            .iter()
            .any(|(name, readable)| kind.and_then(JsValue::as_str) == Some(*name) && *readable);
    let mut fields = if readable && !manifest.is_undefined() {
        manifest_fields(manifest)
    } else {
        JsObject::new()
    };
    fields.extend(entry_fields(entry));
    in_property_order(fields)
}

crate::registry::exports! {
    "format/marketplace/display-fields" {
        "listingFields" => ok |entry: JsValue, manifest: JsValue, stage: ListingStage| listing_fields(&entry, &manifest, stage);
    }
}
