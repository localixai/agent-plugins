//! Port of `src/marketplace/renames.ts`, switched.
//!
//! What a marketplace's `renames` map does to a user who still has an old plugin name (plugins/
//! host-marketplace.md, "Migrate users with a renames map"). Pure decisions over data: the host
//! reads and writes the settings, and shows each notice.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use super::install_scopes::{INSTALL_SCOPES, InstallScope, is_writable_scope};
use super::removal::ListedPlugin;
use super::types::RenameStatus;
use crate::format::marketplace::ids::parse_plugin_ref;
use crate::format::marketplace::renames::{Chain, Renames, follow_renames};
use crate::js::JsValue;

/// The marketplace, as far as the renames read it.
#[derive(Clone, Debug, Deserialize)]
pub struct RenamesMarketplace {
    /// Its name.
    pub name: String,
    /// Its entries.
    pub plugins: Vec<ListedPlugin>,
    /// Old plugin name to the new one, or `null` for a plugin that was removed.
    pub renames: Renames,
}

fn plugin_names(marketplace: &RenamesMarketplace) -> IndexSet<String> {
    marketplace
        .plugins
        .iter()
        .map(|entry| entry.name.clone())
        .collect()
}

/// What each end of a chain means to a user: it loads under the new name, it is gone, or Claude Code has no answer.
fn status_of_chain(chain: Chain) -> RenameStatus {
    match chain {
        Chain::Plugin { name } => RenameStatus::Renamed { to: name },
        Chain::Removed => RenameStatus::Removed,
        Chain::Dangling { .. } | Chain::Cycle { .. } => RenameStatus::Unknown,
    }
}

/// Where `name` stands in `marketplace`: `current` when it is listed (a listed name wins over a key
/// of the same name, judgment); otherwise the chain from the oldest name, followed to the plugin it
/// ends at (`renamed`) or `null` (`removed`); a name the map does not hold, and a chain that does not
/// resolve, are `unknown`, the case that is `Plugin "<name>" not found in marketplace`.
#[must_use]
pub fn resolve_rename(name: &str, marketplace: &RenamesMarketplace) -> RenameStatus {
    let plugins = plugin_names(marketplace);
    if plugins.contains(name) {
        return RenameStatus::Current;
    }
    if !marketplace.renames.contains_key(name) {
        return RenameStatus::Unknown;
    }
    status_of_chain(follow_renames(&marketplace.renames, name, &plugins))
}

/// The text `claude plugin list` and `/plugin` show for a status; nothing for a name that was not renamed or removed.
fn notice_for(status: &RenameStatus, marketplace: &str) -> Option<String> {
    match status {
        RenameStatus::Current | RenameStatus::Unknown => None,
        RenameStatus::Renamed { to } => Some(format!(
            "Renamed to \"{to}\" in the \"{marketplace}\" marketplace"
        )),
        RenameStatus::Removed => Some(format!("Removed from the \"{marketplace}\" marketplace")),
    }
}

/// The plugin name a key is rewritten to; nothing when the key is dropped or stays.
fn new_name_for(status: &RenameStatus) -> Option<&str> {
    match status {
        RenameStatus::Current | RenameStatus::Unknown | RenameStatus::Removed => None,
        RenameStatus::Renamed { to } => Some(to),
    }
}

fn notice_of(status: &RenameStatus, marketplace_name: &str) -> Option<String> {
    notice_for(status, marketplace_name)
}

/// The notice a user sees for `name` in `marketplace`, or nothing when it is current or unknown.
#[must_use]
pub fn rename_notice(name: &str, marketplace: &RenamesMarketplace) -> Option<String> {
    notice_of(&resolve_rename(name, marketplace), &marketplace.name)
}

/// A settings key is `plugin@marketplace`; neither part may hold `@`, so any other shape is not one.
fn split_key(key: &str) -> Option<(String, String)> {
    let reference = parse_plugin_ref(key)?;
    Some((reference.name, reference.marketplace?))
}

/// What `migrateSettingsKeys` answers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MigratedKeys {
    /// The rewritten map.
    pub map: IndexMap<String, JsValue>,
    /// The notices the user sees, each once.
    pub notices: Vec<String>,
}

/// What becomes of one key: it stays (`key` is itself), is dropped (`None`) or is written under the new key.
struct KeyMove {
    key: Option<String>,
    notice: Option<String>,
}

fn move_key(key: &str, marketplace: &RenamesMarketplace) -> KeyMove {
    let stays = KeyMove {
        key: Some(key.to_owned()),
        notice: None,
    };
    let Some((plugin, key_marketplace)) = split_key(key) else {
        return stays;
    };
    if key_marketplace != marketplace.name {
        return stays;
    }
    let status = resolve_rename(&plugin, marketplace);
    let Some(notice) = notice_of(&status, &marketplace.name) else {
        return stays;
    };
    KeyMove {
        key: new_name_for(&status).map(|renamed_to| format!("{renamed_to}@{}", marketplace.name)),
        notice: Some(notice),
    }
}

/// Rewrites the keys of one `enabledPlugins` or `pluginConfigs` map for the marketplace registered
/// as `marketplaceName`: an old name's key becomes the new name's with its value unchanged, the key
/// of a removed plugin is dropped, every other key stays. When the user already has a key under the
/// new name, theirs is kept and the old one dropped (judgment). The notices are those the user sees,
/// each once. When two old keys are rewritten to the same new key and the user has none, the first
/// in map order wins and the other is dropped (judgment). The given map is not changed.
#[must_use]
pub fn migrate_settings_keys(
    map: &IndexMap<String, JsValue>,
    marketplace_name: &str,
    marketplace: &RenamesMarketplace,
) -> MigratedKeys {
    let registered = RenamesMarketplace {
        name: marketplace_name.to_owned(),
        ..marketplace.clone()
    };
    let moves: Vec<(&String, &JsValue, KeyMove)> = map
        .iter()
        .map(|(key, value)| (key, value, move_key(key, &registered)))
        .collect();
    let staying: IndexSet<&String> = moves
        .iter()
        .filter(|(key, _, one)| one.key.as_ref() == Some(*key))
        .map(|(key, _, _)| *key)
        .collect();
    let mut out: IndexMap<String, JsValue> = IndexMap::new();
    for (key, value, one) in &moves {
        let Some(target) = &one.key else {
            continue;
        };
        if out.contains_key(target) {
            continue;
        }
        if target == *key || !staying.contains(target) {
            out.insert(target.clone(), (*value).clone());
        }
    }
    let notices: IndexSet<String> = moves
        .iter()
        .filter_map(|(_, _, one)| one.notice.clone())
        .collect();
    // `fromEntries` defines each key as data, so a `"__proto__"` key stays a key, not a prototype.
    MigratedKeys {
        map: out,
        notices: notices.into_iter().collect(),
    }
}

/// The maps of each scope (`ScopedSettings`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScopedSettings {
    /// The user scope's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<IndexMap<String, JsValue>>,
    /// The project scope's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<IndexMap<String, JsValue>>,
    /// The local scope's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<IndexMap<String, JsValue>>,
    /// The managed scope's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed: Option<IndexMap<String, JsValue>>,
}

impl ScopedSettings {
    fn of(&self, scope: InstallScope) -> Option<&IndexMap<String, JsValue>> {
        match scope {
            InstallScope::User => self.user.as_ref(),
            InstallScope::Project => self.project.as_ref(),
            InstallScope::Local => self.local.as_ref(),
            InstallScope::Managed => self.managed.as_ref(),
        }
    }

    fn set(&mut self, scope: InstallScope, map: IndexMap<String, JsValue>) {
        match scope {
            InstallScope::User => self.user = Some(map),
            InstallScope::Project => self.project = Some(map),
            InstallScope::Local => self.local = Some(map),
            InstallScope::Managed => self.managed = Some(map),
        }
    }
}

/// What `migrateSettingsScopes` answers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MigratedScopes {
    /// The scopes, rewritten where they can be.
    pub scopes: ScopedSettings,
    /// The notices, each once.
    pub notices: Vec<String>,
}

/// `migrateSettingsKeys` over the maps of each scope: user, project and local are rewritten; managed
/// settings cannot be, so they come back as given and their notice is given again on every call
/// until an admin updates them. A notice that several scopes give is given once.
#[must_use]
pub fn migrate_settings_scopes(
    scopes: &ScopedSettings,
    marketplace_name: &str,
    marketplace: &RenamesMarketplace,
) -> MigratedScopes {
    let mut out = ScopedSettings::default();
    let mut notices: IndexSet<String> = IndexSet::new();
    for scope in INSTALL_SCOPES {
        let Some(given) = scopes.of(scope) else {
            continue;
        };
        let migrated = migrate_settings_keys(given, marketplace_name, marketplace);
        out.set(
            scope,
            if is_writable_scope(scope) {
                migrated.map
            } else {
                given.clone()
            },
        );
        notices.extend(migrated.notices);
    }
    MigratedScopes {
        scopes: out,
        notices: notices.into_iter().collect(),
    }
}

crate::registry::exports! {
    "marketplace/renames" {
        "resolveRename" => ok |name: String, marketplace: RenamesMarketplace| resolve_rename(&name, &marketplace);
        "renameNotice" => ok |name: String, marketplace: RenamesMarketplace| rename_notice(&name, &marketplace);
        "migrateSettingsKeys" => ok |map: IndexMap<String, JsValue>, marketplace_name: String, marketplace: RenamesMarketplace| migrate_settings_keys(&map, &marketplace_name, &marketplace);
        "migrateSettingsScopes" => ok |scopes: ScopedSettings, marketplace_name: String, marketplace: RenamesMarketplace| migrate_settings_scopes(&scopes, &marketplace_name, &marketplace);
    }
}
