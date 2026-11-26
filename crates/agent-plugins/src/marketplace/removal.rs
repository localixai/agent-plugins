//! Port of `src/marketplace/removal.ts`, switched.
//!
//! `forceRemoveDeletedPlugins` (plugins/host-marketplace.md, "Uninstall removed plugins from users'
//! machines"): at each session start Claude Code "compares what users installed from your
//! marketplace against the entries and the `renames` map, and treats any plugin that is neither
//! listed nor renamed as removed", uninstalls it from the user, project and local scopes (what only
//! managed settings installed stays in place) and lists it as `Removed from marketplace`. The kit
//! decides what is removed; the host does the uninstalling.

use serde::{Deserialize, Serialize};

use super::install_scopes::{InstallScope, is_writable_scope};
use crate::format::marketplace::renames::Renames;

/// A plugin the user has installed, and the scopes it is installed or enabled in.
#[derive(Clone, Debug, Deserialize)]
pub struct InstalledPlugin {
    /// Its name.
    pub name: String,
    /// Where it came from.
    pub marketplace: String,
    /// The scopes it is in.
    pub scopes: Vec<InstallScope>,
}

/// The status `/plugin` shows under its **Flagged** heading.
pub const REMOVED_STATUS: &str = "Removed from marketplace";

/// A plugin that was removed from its marketplace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovedPlugin {
    /// `name@marketplace`.
    pub id: String,
    /// `REMOVED_STATUS`.
    pub status: &'static str,
    /// The scopes to uninstall from: every scope it is in but `managed`, which the user cannot change.
    pub uninstall_from: Vec<InstallScope>,
}

/// A marketplace entry, as far as the removal reads it.
#[derive(Clone, Debug, Deserialize)]
pub struct ListedPlugin {
    /// The plugin's name.
    pub name: String,
}

/// The marketplace, as far as the removal reads it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovalMarketplace {
    /// Its name.
    pub name: String,
    /// Its entries.
    pub plugins: Vec<ListedPlugin>,
    /// Whether a plugin that left the list is removed.
    pub force_remove_deleted_plugins: bool,
    /// Old plugin name to the new one, or `null` for a plugin that was removed.
    pub renames: Renames,
}

/// Judgments (spec/judgments.md): only plugins installed from `marketplace` are considered; "renamed"
/// means a key of `renames` with a name as its target, even when its chain does not resolve, so a
/// map the marketplace got wrong never uninstalls on doubt; a `null` entry is a removal (the docs'
/// "mark a plugin deprecated" is that entry plus the flag); a listed name is never removed; a plugin
/// installed only by managed settings is still reported (it is treated as removed and flagged) with
/// nothing to uninstall.
#[must_use]
pub fn removed_plugins(
    installed: &[InstalledPlugin],
    marketplace: &RemovalMarketplace,
) -> Vec<RemovedPlugin> {
    if !marketplace.force_remove_deleted_plugins {
        return Vec::new();
    }
    let listed: Vec<&str> = marketplace
        .plugins
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let renamed = |name: &str| marketplace.renames.get(name).is_some_and(Option::is_some);
    installed
        .iter()
        .filter(|one| one.marketplace == marketplace.name)
        .filter(|one| !listed.contains(&one.name.as_str()) && !renamed(&one.name))
        .map(|one| RemovedPlugin {
            id: format!("{}@{}", one.name, one.marketplace),
            status: REMOVED_STATUS,
            uninstall_from: one
                .scopes
                .iter()
                .copied()
                .filter(|scope| is_writable_scope(*scope))
                .collect(),
        })
        .collect()
}

crate::registry::exports! {
    "marketplace/removal" {
        "REMOVED_STATUS" => ok || REMOVED_STATUS;
        "removedPlugins" => ok |installed: Vec<InstalledPlugin>, marketplace: RemovalMarketplace| removed_plugins(&installed, &marketplace);
    }
}
