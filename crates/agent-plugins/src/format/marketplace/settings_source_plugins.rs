//! Ported from `src/format/marketplace/settings-source-plugins.ts`; only the core uses it now, so it has no wrapper.
//!
//! The inline catalog of a `settings` marketplace source (plugins/marketplace-reference.md, "Fields
//! by type": "Each item takes `name`, `source`, `description`, `version`, `strict`, `headers`, and
//! `headersHelper`. Write each item's `source` as an object type, because a relative path has no
//! repository to resolve against"; settings-reference.md, `extraKnownMarketplaces`: "an entry in
//! settings doesn't need `"strict": false`, because a settings file carries no manifest fields to
//! inline"). Issue paths are `plugins[i]...`, relative to the source object.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use super::entries::{EntrySlot, read_entry};
use super::entry_checks::{IndexedEntry, without_duplicates};
use super::entry_schema::ENTRY_KEYS;
use super::marketplace_schema::{mismatch, unknown_field};
use super::plugin_source::parse_plugin_source;
use super::types::{MarketplaceContext, MarketplaceEntry, PluginSourceKind, ReadMode};
use crate::format::issues::{PluginIssue, Severity, blocks_loading, error};
use crate::js::{JsObject, JsValue, Pattern, property_order, regex_test};
use crate::ports::now;
use crate::protocol::is_record::is_record;

/// The seven keys an item takes; any other is unknown and ignored (spec/judgments.md).
const SETTINGS_ENTRY_KEYS: [&str; 7] = [
    "name",
    "source",
    "description",
    "version",
    "strict",
    "headers",
    "headersHelper",
];

fn is_item_key(key: &str) -> bool {
    SETTINGS_ENTRY_KEYS.contains(&key)
}

const RELATIVE_REFUSAL: &str = "A relative path cannot resolve: an entry declared in settings has no marketplace repository. Give it a github, url, git-subdir, npm, archive or command source";

/// Which plugin sources are paths inside a marketplace repository: the one kind settings cannot hold.
const fn needs_repository(kind: PluginSourceKind) -> bool {
    match kind {
        PluginSourceKind::Relative => true,
        PluginSourceKind::Github
        | PluginSourceKind::Url
        | PluginSourceKind::GitSubdir
        | PluginSourceKind::Npm
        | PluginSourceKind::Archive
        | PluginSourceKind::Command => false,
    }
}

/// Whether the item must be read as `strict: false`: the entry reader refuses `headersHelper`
/// without it, a rule of `marketplace.json` entries that does not apply in settings. The entry keeps
/// the `strict` the item declared.
fn needs_relaxed_strict(item: &JsObject) -> bool {
    let strict = item.get("strict").unwrap_or(&JsValue::Undefined);
    item.get("headersHelper")
        .is_some_and(|helper| !helper.is_undefined())
        && (strict.is_undefined() || *strict == JsValue::Bool(true))
}

static CATALOG_HEADER: Pattern =
    LazyLock::new(|| Regex::new(r"^plugins\[[0-9]+\]\.headers\.[^.]+$").ok());

/// The catalog warning about routing headers names a rule of catalog entries; in settings it depends on the file (D-download-headers).
fn is_catalog_header_notice(found: &PluginIssue) -> bool {
    found.severity == Severity::Warning && regex_test(&CATALOG_HEADER, &found.path)
}

struct ItemReading {
    entry: Option<MarketplaceEntry>,
    issues: Vec<PluginIssue>,
}

fn read_item(item: &JsObject, index: usize) -> ItemReading {
    let mut known: JsObject = property_order(item)
        .into_iter()
        .filter(|(key, _)| is_item_key(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let keys: Vec<&str> = ENTRY_KEYS.iter().map(String::as_str).collect();
    let unknown = property_order(item)
        .into_iter()
        .filter(|(key, _)| !is_item_key(key))
        .map(|(key, _)| unknown_field(key, &format!("plugins[{index}].{key}"), &keys));
    if needs_relaxed_strict(item) {
        known.insert("strict".to_owned(), JsValue::Bool(false));
    }
    let slot = EntrySlot {
        index: u32::try_from(index).unwrap_or(u32::MAX),
        plugin_root: None,
        context: MarketplaceContext::default(),
        mode: Some(ReadMode::Load),
    };
    let parser = parse_plugin_source;
    let raw = JsValue::Object(known);
    let mut issues: Vec<PluginIssue> = unknown.collect();
    // The parser is the core's own, so the reading is ready at once.
    let Some(reading) = now(read_entry(&raw, &slot, &parser)) else {
        return ItemReading {
            entry: None,
            issues,
        };
    };
    issues.extend(
        reading
            .issues
            .into_iter()
            .filter(|found| !is_catalog_header_notice(found)),
    );
    let Some(entry) = reading.entry else {
        return ItemReading {
            entry: None,
            issues,
        };
    };
    if needs_repository(entry.source.kind()) {
        issues.push(error(&format!("plugins[{index}].source"), RELATIVE_REFUSAL));
        return ItemReading {
            entry: None,
            issues,
        };
    }
    let strict = match item.get("strict") {
        Some(JsValue::Bool(strict)) => *strict,
        _ => true,
    };
    ItemReading {
        entry: Some(MarketplaceEntry { strict, ..entry }),
        issues,
    }
}

/// `readSettingsPlugins`' answer.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SettingsPlugins {
    /// The entries that load.
    pub plugins: Vec<MarketplaceEntry>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// The entries that load and what was found; an item that cannot load is dropped alone.
#[must_use]
pub fn read_settings_plugins(items: &[JsValue]) -> SettingsPlugins {
    let mut kept: Vec<IndexedEntry> = Vec::new();
    let mut issues: Vec<PluginIssue> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(members) = item.as_object().filter(|_| is_record(item)) else {
            issues.push(mismatch(&format!("plugins[{index}]"), "object", item));
            continue;
        };
        let reading = read_item(members, index);
        let blocks = blocks_loading(&reading.issues);
        issues.extend(reading.issues);
        if let Some(entry) = reading.entry.filter(|_| !blocks) {
            kept.push(IndexedEntry { index, entry });
        }
    }
    let unique = without_duplicates(&kept);
    issues.extend(unique.issues);
    SettingsPlugins {
        plugins: unique.entries,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::{CATALOG_HEADER, regex_test};

    #[test]
    fn the_catalog_header_pattern_compiles_and_reads_one_header() {
        assert!(CATALOG_HEADER.is_some());
        assert!(regex_test(&CATALOG_HEADER, "plugins[3].headers.X-Key"));
        assert!(!regex_test(&CATALOG_HEADER, "plugins[3].headers.a.b"));
    }
}
