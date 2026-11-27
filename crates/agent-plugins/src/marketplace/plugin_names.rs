//! Port of `src/marketplace/plugin-names.ts`, switched.
//!
//! The two names of a marketplace plugin (plugins/loading.md, "Entry name and manifest name";
//! plugins/create-marketplace.md, "Keep the entry name and the manifest name the same"). The entry
//! name in `marketplace.json` is "the install and enable key": what goes in `enabledPlugins`, what
//! the cache directory is named after and what `claude plugin list` shows. The `name` in the manifest
//! is "what the plugin's components are namespaced under, and what name conflicts compare".

use serde::Serialize;

use super::install_errors::{PluginNotFound, Refresh, plugin_not_found};
use crate::format::marketplace::entry_merge::EntryMerge;
use crate::format::marketplace::types::MarketplaceEntry;

/// The names of a plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginNames {
    /// `<entry name>@<marketplace>`: the install id, the `enabledPlugins` key and the id `claude plugin list` shows.
    pub install_id: String,
    /// The `<plugin>` of `cache/<marketplace>/<plugin>/<version>/`.
    pub cache_name: String,
    /// The manifest name (the entry name when there is no `plugin.json`): the components' namespace and the name conflicts compare.
    pub namespace: String,
}

/// The names of a plugin from its entry merge; a plugin that fails to load keeps its entry name, as it still has an id.
#[must_use]
pub fn plugin_names(merge: &EntryMerge, marketplace: &str) -> PluginNames {
    PluginNames {
        install_id: format!("{}@{marketplace}", merge.install_name),
        cache_name: merge.install_name.clone(),
        namespace: merge
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or(&merge.install_name)
            .to_owned(),
    }
}

/// The entry found, or why there is none.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginLookup {
    /// `{ entry }`.
    Entry(Box<MarketplaceEntry>),
    /// `{ notFound }`.
    NotFound(PluginNotFound),
}

/// Installing looks a plugin up by its entry name only: "When the two names differ and someone
/// installs by the manifest name, Claude Code reports `Plugin "<manifest-name>" not found in
/// marketplace "<marketplace>"`." The catalog is entries, so a manifest name never matches.
#[must_use]
pub fn lookup_plugin(
    entries: &[MarketplaceEntry],
    name: &str,
    marketplace: &str,
    refresh: &Refresh,
) -> PluginLookup {
    entries.iter().find(|one| one.name == name).map_or_else(
        || PluginLookup::NotFound(plugin_not_found(name, marketplace, refresh)),
        |entry| PluginLookup::Entry(Box::new(entry.clone())),
    )
}

crate::registry::exports! {
    "marketplace/plugin-names" {
        "pluginNames" => ok |merge: EntryMerge, marketplace: String| plugin_names(&merge, &marketplace);
        "lookupPlugin" => ok |entries: Vec<MarketplaceEntry>, name: String, marketplace: String, refresh: Refresh|
            lookup_plugin(&entries, &name, &marketplace, &refresh);
    }
}
