//! Port of `src/marketplace/store-layout.ts`, switched.
//!
//! Where Claude Code keeps plugin files and state records (plugins/loading.md, "Find plugins on
//! disk"; env-vars.md, `CLAUDE_CODE_PLUGIN_CACHE_DIR`): one plugins root, and under it the version
//! cache, the persistent data folders, the marketplace clones, the claude.ai sync folders, the JSON
//! records and the dated copies Claude Code keeps before it rewrites `installed_plugins.json`.
//!
//! Pure path builders over a host-supplied `join` (the host's own separator, as `Locations.joinPath`):
//! the kit never concatenates a path with its own separator and uses no `node:path` (L3). The
//! `join` is the host's function, so every builder may wait on it.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::config_directory::{Join, config_directory_of};
use crate::js::Pattern;
use crate::registry::{Callback, Handle, Port};

/// The plugins root of a default install, relative to the home directory.
const PLUGINS_FOLDER: &str = "plugins";

/// What `resolvePluginsRoot` reads.
pub struct PluginsRootOptions<'a> {
    /// The home directory.
    pub home: &'a str,
    /// `CLAUDE_CODE_PLUGIN_CACHE_DIR`.
    pub override_root: Option<&'a str>,
    /// `CLAUDE_CONFIG_DIR`: the directory every `~/.claude` path lives under when it is set.
    pub config_dir: Option<&'a str>,
    /// The host's join.
    pub join: &'a dyn Join,
}

/// The plugins root: `CLAUDE_CODE_PLUGIN_CACHE_DIR` when set, else `plugins` under the configuration
/// directory (`CLAUDE_CONFIG_DIR`: "All settings, session history, and plugins are stored under this
/// path"), else `~/.claude/plugins`. "Despite the name, this sets the parent directory, not the cache
/// itself" (env-vars.md). Judgments (spec/judgments.md): the docs do not say which wins when both
/// variables are set, so the one that names the plugins root itself wins over the configuration
/// directory; an empty value is no override, as for an unset variable.
pub async fn resolve_plugins_root(options: &PluginsRootOptions<'_>) -> String {
    if let Some(override_root) = options.override_root.filter(|root| !root.is_empty()) {
        return override_root.to_owned();
    }
    let directory = config_directory_of(options.home, options.config_dir, options.join).await;
    options.join.join(&directory, PLUGINS_FOLDER).await
}

/// The names the table of "Find plugins on disk" gives; each is directly under the root.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreNames {
    /// `cache`.
    pub cache: &'static str,
    /// `data`.
    pub data: &'static str,
    /// `marketplaces`.
    pub marketplaces: &'static str,
    /// `synced`.
    pub synced: &'static str,
    /// `.trash`.
    pub trash: &'static str,
    /// `installed_plugins.json`.
    pub installed_plugins: &'static str,
    /// `known_marketplaces.json`.
    pub known_marketplaces: &'static str,
    /// `known_marketplaces_claudeai.json`.
    pub known_marketplaces_claude_ai: &'static str,
    /// `flagged-plugins.json`.
    pub flagged_plugins: &'static str,
}

/// The names the table of "Find plugins on disk" gives; each is directly under the root.
pub const STORE_NAMES: StoreNames = StoreNames {
    cache: "cache",
    data: "data",
    marketplaces: "marketplaces",
    synced: "synced",
    trash: ".trash",
    installed_plugins: "installed_plugins.json",
    known_marketplaces: "known_marketplaces.json",
    known_marketplaces_claude_ai: "known_marketplaces_claudeai.json",
    flagged_plugins: "flagged-plugins.json",
};

const SET_ASIDE_PREFIX: &str = "installed_plugins.set-aside.";
const UNREADABLE_PREFIX: &str = "installed_plugins.unreadable.";

/// The layout under `root`. Judgment (spec/judgments.md): `<date>` and `<hash>` of the dated copies
/// are the caller's text (the docs give the shape, not the date format or the hash input).
pub struct StoreLayout<J> {
    /// The plugins root.
    pub root: String,
    /// How a path is joined under it.
    pub join: J,
}

impl<J: Join> StoreLayout<J> {
    async fn under(&self, parts: &[&str]) -> String {
        self.join.join(&self.root, &parts.join("/")).await
    }

    /// A file or folder directly under the root, by name (a name listed in `STORE_NAMES` or a dated copy).
    pub async fn file(&self, name: &str) -> String {
        self.under(&[name]).await
    }

    /// `cache/`.
    pub async fn cache(&self) -> String {
        self.under(&[STORE_NAMES.cache]).await
    }

    /// `cache/<marketplace>/<plugin>/<version>/`: `${CLAUDE_PLUGIN_ROOT}` of a copied marketplace plugin.
    pub async fn version_dir(&self, marketplace: &str, plugin: &str, version: &str) -> String {
        self.under(&[STORE_NAMES.cache, marketplace, plugin, version])
            .await
    }

    /// `data/<plugin-id>/`; `dataId` is the sanitized `<plugin-id>` (`pluginDataId`, `loading/resolve`).
    pub async fn plugin_data(&self, data_id: &str) -> String {
        self.under(&[STORE_NAMES.data, data_id]).await
    }

    /// `marketplaces/<name>/`: the clone or download of a GitHub, git or URL marketplace.
    pub async fn marketplace(&self, name: &str) -> String {
        self.under(&[STORE_NAMES.marketplaces, name]).await
    }

    /// `synced/`.
    pub async fn synced(&self) -> String {
        self.under(&[STORE_NAMES.synced]).await
    }

    /// `.trash/`.
    pub async fn trash(&self) -> String {
        self.under(&[STORE_NAMES.trash]).await
    }

    /// `installed_plugins.json`.
    pub async fn installed_plugins(&self) -> String {
        self.under(&[STORE_NAMES.installed_plugins]).await
    }

    /// `known_marketplaces.json`.
    pub async fn known_marketplaces(&self) -> String {
        self.under(&[STORE_NAMES.known_marketplaces]).await
    }

    /// A marketplace hosted on claude.ai is recorded here instead of in `known_marketplaces.json`.
    pub async fn known_marketplaces_claude_ai(&self) -> String {
        self.under(&[STORE_NAMES.known_marketplaces_claude_ai])
            .await
    }

    /// `flagged-plugins.json`.
    pub async fn flagged_plugins(&self) -> String {
        self.under(&[STORE_NAMES.flagged_plugins]).await
    }

    /// The copy of the install records dropped for names no version can use.
    pub async fn set_aside_copy(&self, date: &str, hash: &str) -> String {
        self.under(&[&format!("{SET_ASIDE_PREFIX}{date}.{hash}.json")])
            .await
    }

    /// The copy of an `installed_plugins.json` that could not be read.
    pub async fn unreadable_copy(&self, date: &str, hash: &str) -> String {
        self.under(&[&format!("{UNREADABLE_PREFIX}{date}.{hash}.kept")])
            .await
    }
}

/// The layout under `root`.
pub const fn create_store_layout<J: Join>(root: String, join: J) -> StoreLayout<J> {
    StoreLayout { root, join }
}

/// What a name directly under the plugins root is, as the table of "Find plugins on disk" lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoreEntry {
    /// `cache`.
    Cache,
    /// `data`.
    Data,
    /// `marketplaces`.
    Marketplaces,
    /// `synced`.
    Synced,
    /// `.trash`.
    Trash,
    /// `installed_plugins.json`.
    InstalledPlugins,
    /// `known_marketplaces.json`.
    KnownMarketplaces,
    /// `known_marketplaces_claudeai.json`.
    #[serde(rename = "known-marketplaces-claudeai")]
    KnownMarketplacesClaudeAi,
    /// `flagged-plugins.json`.
    FlaggedPlugins,
    /// A dated set-aside copy.
    SetAsideCopy,
    /// A dated unreadable copy.
    UnreadableCopy,
    /// Anything else.
    Unknown,
}

/// `/^installed_plugins\.set-aside\.[^./]+\.[^./]+\.json$/`.
static SET_ASIDE_NAME: Pattern =
    LazyLock::new(|| Regex::new(r"^installed_plugins\.set-aside\.[^./]+\.[^./]+\.json$").ok());
/// `/^installed_plugins\.unreadable\.[^./]+\.[^./]+\.kept$/`.
static UNREADABLE_NAME: Pattern =
    LazyLock::new(|| Regex::new(r"^installed_plugins\.unreadable\.[^./]+\.[^./]+\.kept$").ok());

const EXACT: [(&str, StoreEntry); 9] = [
    (STORE_NAMES.cache, StoreEntry::Cache),
    (STORE_NAMES.data, StoreEntry::Data),
    (STORE_NAMES.marketplaces, StoreEntry::Marketplaces),
    (STORE_NAMES.synced, StoreEntry::Synced),
    (STORE_NAMES.trash, StoreEntry::Trash),
    (STORE_NAMES.installed_plugins, StoreEntry::InstalledPlugins),
    (
        STORE_NAMES.known_marketplaces,
        StoreEntry::KnownMarketplaces,
    ),
    (
        STORE_NAMES.known_marketplaces_claude_ai,
        StoreEntry::KnownMarketplacesClaudeAi,
    ),
    (STORE_NAMES.flagged_plugins, StoreEntry::FlaggedPlugins),
];

fn matches(pattern: &Pattern, name: &str) -> bool {
    pattern.as_ref().is_some_and(|regex| regex.is_match(name))
}

/// What `name` is.
#[must_use]
pub fn classify_store_entry(name: &str) -> StoreEntry {
    if let Some((_, entry)) = EXACT.iter().find(|(exact, _)| *exact == name) {
        return *entry;
    }
    if matches(&SET_ASIDE_NAME, name) {
        return StoreEntry::SetAsideCopy;
    }
    if matches(&UNREADABLE_NAME, name) {
        StoreEntry::UnreadableCopy
    } else {
        StoreEntry::Unknown
    }
}

/// `resolvePluginsRoot`'s options as the host gave them: a record that holds its `join`.
async fn resolved_root(options: Port) -> String {
    let join = options.member::<Callback>("join").await;
    let home = options.member::<String>("home").await;
    let override_root = options.member::<Option<String>>("override").await;
    let config_dir = options.member::<Option<String>>("configDir").await;
    resolve_plugins_root(&PluginsRootOptions {
        home: &home,
        override_root: override_root.as_deref(),
        config_dir: config_dir.as_deref(),
        join: &join,
    })
    .await
}

crate::registry::methods! {
    StoreLayout<Callback> {
        "file" => sync async ok |this, name: String| this.file(&name).await;
        "cache" => sync async ok |this| this.cache().await;
        "versionDir" => sync async ok |this, marketplace: String, plugin: String, version: String|
            this.version_dir(&marketplace, &plugin, &version).await;
        "pluginData" => sync async ok |this, data_id: String| this.plugin_data(&data_id).await;
        "marketplace" => sync async ok |this, name: String| this.marketplace(&name).await;
        "synced" => sync async ok |this| this.synced().await;
        "trash" => sync async ok |this| this.trash().await;
        "installedPlugins" => sync async ok |this| this.installed_plugins().await;
        "knownMarketplaces" => sync async ok |this| this.known_marketplaces().await;
        "knownMarketplacesClaudeAi" => sync async ok |this| this.known_marketplaces_claude_ai().await;
        "flaggedPlugins" => sync async ok |this| this.flagged_plugins().await;
        "setAsideCopy" => sync async ok |this, date: String, hash: String|
            this.set_aside_copy(&date, &hash).await;
        "unreadableCopy" => sync async ok |this, date: String, hash: String|
            this.unreadable_copy(&date, &hash).await;
    }
}

crate::registry::exports! {
    "marketplace/store-layout" {
        "resolvePluginsRoot" => async ok |options: Port| resolved_root(options).await;
        "STORE_NAMES" => ok || STORE_NAMES;
        "createStoreLayout" => ok |root: String, join: Callback|
            Handle::new(create_store_layout(root, join));
        "classifyStoreEntry" => ok |name: String| classify_store_entry(&name);
    }
}
