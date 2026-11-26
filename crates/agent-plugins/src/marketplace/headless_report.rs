//! Port of `src/marketplace/headless-report.ts`, switched.
//!
//! What a non-interactive run reports about plugins in its stream output (headless.md, "Fail CI when
//! a plugin or MCP server doesn't load", "Track plugin installs"; the SDK's `SDKSystemMessage` and
//! `SDKPluginInstallMessage`). The kit builds the fields from what the host loaded and installed;
//! writing the events to the stream, and when, is the host's.

use serde::{Deserialize, Serialize};

use crate::js::number_to_string;
use crate::ports::BoxFuture;
use crate::registry::{Callback, Port};

/// A load-time error: a category from an open set (`path-not-found`, `dependency-unsatisfied`, ...) and display text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginLoadError {
    /// The category.
    pub r#type: String,
    /// The display text.
    pub message: String,
}

/// A plugin the session loaded; `errors` are the components it loaded without.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedPluginRow {
    /// `<name>@<marketplace>`.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Where it was loaded from.
    pub path: String,
    /// The manifest's `version`, verbatim; absent when it declares none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The components it loaded without.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<PluginLoadError>>,
}

/// A plugin that did not load, such as one whose dependency's version is not satisfied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginNotLoaded {
    /// `<name>@<marketplace>`.
    pub id: String,
    /// Why.
    pub error: PluginLoadError,
}

/// Where a failed directory entry came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectoryOrigin {
    /// A `--plugin-dir`, `--plugin-url` or SDK `plugins` entry.
    Inline,
    /// A synced directory entry.
    Synced,
}

impl DirectoryOrigin {
    /// The word the entry is reported by.
    const fn word(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Synced => "synced",
        }
    }
}

/// A `--plugin-dir`, SDK `plugins` or synced directory entry that failed before it had a name: it is
/// reported by its position (`inline[N]`, `synced[N]`) and the path it resolved to, absolute.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct FailedPluginDirectory {
    /// Where it came from.
    pub origin: DirectoryOrigin,
    /// Its position.
    pub index: f64,
    /// Absent for a `--plugin-url` entry.
    #[serde(default)]
    pub path: Option<String>,
    /// Why.
    pub error: PluginLoadError,
}

/// What the host loaded and what failed.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitPluginFacts {
    /// The plugins that loaded.
    pub loaded: Vec<LoadedPluginRow>,
    /// The plugins that did not.
    #[serde(default)]
    pub unloaded: Option<Vec<PluginNotLoaded>>,
    /// The directory entries that failed before they had a name.
    #[serde(default)]
    pub failed_directories: Option<Vec<FailedPluginDirectory>>,
    /// The session's frames are persisted server-side (a Remote Control worker): `plugin_errors` is always omitted.
    #[serde(default)]
    pub persisted_remotely: Option<bool>,
}

/// One row of `plugins`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InitPluginRow {
    /// Its name.
    pub name: String,
    /// Its path.
    pub path: String,
    /// Its version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// One row of `plugin_errors`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InitPluginError {
    /// The plugin, or `inline[N]` / `synced[N]`.
    pub plugin: String,
    /// The category.
    pub r#type: String,
    /// The display text.
    pub message: String,
    /// The path a failed directory resolved to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// The SDK's `InitPluginFields`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InitPluginFields {
    /// What loaded.
    pub plugins: Vec<InitPluginRow>,
    /// The load-time errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_errors: Option<Vec<InitPluginError>>,
}

fn row_of(plugin: &LoadedPluginRow) -> InitPluginRow {
    InitPluginRow {
        name: plugin.name.clone(),
        path: plugin.path.clone(),
        version: plugin.version.clone(),
    }
}

fn error_of(plugin: &str, error: &PluginLoadError, path: Option<&String>) -> InitPluginError {
    InitPluginError {
        plugin: plugin.to_owned(),
        r#type: error.r#type.clone(),
        message: error.message.clone(),
        path: path.cloned(),
    }
}

fn directory_error(entry: &FailedPluginDirectory) -> InitPluginError {
    error_of(
        &format!("{}[{}]", entry.origin.word(), number_to_string(entry.index)),
        &entry.error,
        entry.path.as_ref(),
    )
}

/// `plugins` lists what loaded, each with `name` and `path`; `plugin_errors` the load-time errors,
/// each with `plugin`, `type` and `message`, and the key "is omitted when there are no errors". A
/// plugin that did not load is absent from `plugins`; one that loaded without a component keeps its
/// row and gets an entry too.
#[must_use]
pub fn init_plugin_fields(facts: &InitPluginFacts) -> InitPluginFields {
    let mut errors: Vec<InitPluginError> = Vec::new();
    for plugin in &facts.loaded {
        for error in plugin.errors.iter().flatten() {
            errors.push(error_of(&plugin.id, error, None));
        }
    }
    for plugin in facts.unloaded.iter().flatten() {
        errors.push(error_of(&plugin.id, &plugin.error, None));
    }
    errors.extend(
        facts
            .failed_directories
            .iter()
            .flatten()
            .map(directory_error),
    );
    let omitted = errors.is_empty() || facts.persisted_remotely == Some(true);
    InitPluginFields {
        plugins: facts.loaded.iter().map(row_of).collect(),
        plugin_errors: if omitted { None } else { Some(errors) },
    }
}

/// How one marketplace's install ended: `error` when it failed.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MarketplaceInstallOutcome {
    /// The marketplace.
    pub name: String,
    /// Why it failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// A fresh event identifier per call: the kit has no source of randomness (L3). A function of the
/// host's, so its answer may wait: a Rust host passes a closure, answered at once.
pub trait NextUuid: Send + Sync {
    /// A fresh identifier.
    fn uuid(&self) -> BoxFuture<'_, String>;
}

impl<F: Fn() -> String + Send + Sync> NextUuid for F {
    fn uuid(&self) -> BoxFuture<'_, String> {
        Box::pin(std::future::ready(self()))
    }
}

impl NextUuid for Callback {
    fn uuid(&self) -> BoxFuture<'_, String> {
        Box::pin(self.call::<String>(Vec::new()))
    }
}

/// What stamps an event.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallStamp<U> {
    /// The session's id.
    pub session_id: String,
    /// A fresh event identifier per call.
    pub uuid: U,
}

/// The SDK's `SDKPluginInstallMessage`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PluginInstallMessage {
    /// `system`.
    pub r#type: &'static str,
    /// `plugin_install`.
    pub subtype: &'static str,
    /// `started`, `installed`, `failed` or `completed`.
    pub status: &'static str,
    /// The marketplace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Why it failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The event's identifier.
    pub uuid: String,
    /// The session's id.
    pub session_id: String,
}

/// One event of the install.
async fn event<U: NextUuid>(
    stamp: &PluginInstallStamp<U>,
    status: &'static str,
    name: Option<&str>,
    error: Option<&str>,
) -> PluginInstallMessage {
    PluginInstallMessage {
        r#type: "system",
        subtype: "plugin_install",
        status,
        name: name.map(str::to_owned),
        error: error.map(str::to_owned),
        uuid: stamp.uuid.uuid().await,
        session_id: stamp.session_id.clone(),
    }
}

/// The `system/plugin_install` events of a synchronous install (`CLAUDE_CODE_SYNC_PLUGIN_INSTALL`):
/// `started` and `completed` bracket the install, `installed` and `failed` report one marketplace
/// each, with `name` and, on `failed`, `error`. Whether the install waits is `syncPluginInstall`'s.
pub async fn plugin_install_events<U: NextUuid>(
    outcomes: &[MarketplaceInstallOutcome],
    stamp: &PluginInstallStamp<U>,
) -> Vec<PluginInstallMessage> {
    let mut events = vec![event(stamp, "started", None, None).await];
    for outcome in outcomes {
        events.push(match outcome.error.as_deref() {
            None => event(stamp, "installed", Some(&outcome.name), None).await,
            Some(error) => event(stamp, "failed", Some(&outcome.name), Some(error)).await,
        });
    }
    events.push(event(stamp, "completed", None, None).await);
    events
}

/// `PluginInstallStamp` as it crosses: a record holding a function, so a port whose members are
/// read (docs/RUST.md, "Effects").
async fn install_stamp(record: Port) -> PluginInstallStamp<Callback> {
    let session_id = record.member("sessionId");
    let uuid = record.member("uuid");
    PluginInstallStamp {
        session_id: session_id.await,
        uuid: uuid.await,
    }
}

crate::registry::exports! {
    "marketplace/headless-report" {
        "initPluginFields" => ok |facts: InitPluginFacts| init_plugin_fields(&facts);
        "pluginInstallEvents" => async ok |outcomes: Vec<MarketplaceInstallOutcome>, stamp: Port|
            plugin_install_events(&outcomes, &install_stamp(stamp).await).await;
    }
}
