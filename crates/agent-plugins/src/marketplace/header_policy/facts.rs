//! The vocabulary of `header-policy.ts`: the places, origins and operations a `headers` or
//! `headersHelper` declaration has, and the layer and setting the policy reads.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::path_text::PathPlatform;

/// The two places `headers` and `headersHelper` can be set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeaderPlace {
    /// `url-source`.
    #[serde(rename = "url-source")]
    UrlSource,
    /// `entry`.
    #[serde(rename = "entry")]
    Entry,
}

/// The two places `headers` and `headersHelper` can be set.
pub const HEADER_PLACES: [HeaderPlace; 2] = [HeaderPlace::UrlSource, HeaderPlace::Entry];

/// Where the declaration lives: a `marketplace.json` entry (`catalog`), or a settings file: user
/// settings, a `--settings` file, a managed settings file on the machine, server-managed settings,
/// a project's `.claude/settings.json` or `settings.local.json`, or an `--add-dir` directory's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeaderOrigin {
    /// `catalog`.
    #[serde(rename = "catalog")]
    Catalog,
    /// `user`.
    #[serde(rename = "user")]
    User,
    /// `cli-settings`.
    #[serde(rename = "cli-settings")]
    CliSettings,
    /// `managed`.
    #[serde(rename = "managed")]
    Managed,
    /// `server-managed`.
    #[serde(rename = "server-managed")]
    ServerManaged,
    /// `project`.
    #[serde(rename = "project")]
    Project,
    /// `add-dir`.
    #[serde(rename = "add-dir")]
    AddDir,
}

/// Where the declaration lives.
pub const HEADER_ORIGINS: [HeaderOrigin; 7] = [
    HeaderOrigin::Catalog,
    HeaderOrigin::User,
    HeaderOrigin::CliSettings,
    HeaderOrigin::Managed,
    HeaderOrigin::ServerManaged,
    HeaderOrigin::Project,
    HeaderOrigin::AddDir,
];

/// What the user is doing when an entry's archive is about to be downloaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginOperation {
    /// `install`.
    #[serde(rename = "install")]
    Install,
    /// `update`.
    #[serde(rename = "update")]
    Update,
    /// `bulk-install`.
    #[serde(rename = "bulk-install")]
    BulkInstall,
    /// `suggestion`.
    #[serde(rename = "suggestion")]
    Suggestion,
    /// `dependency`.
    #[serde(rename = "dependency")]
    Dependency,
    /// `background-update`.
    #[serde(rename = "background-update")]
    BackgroundUpdate,
    /// `session-start`.
    #[serde(rename = "session-start")]
    SessionStart,
}

/// What the user is doing when an entry's archive is about to be downloaded.
pub const PLUGIN_OPERATIONS: [PluginOperation; 7] = [
    PluginOperation::Install,
    PluginOperation::Update,
    PluginOperation::BulkInstall,
    PluginOperation::Suggestion,
    PluginOperation::Dependency,
    PluginOperation::BackgroundUpdate,
    PluginOperation::SessionStart,
];

/// The command and archive URL the user was shown and accepted. `query_counts` is set for the VS Code
/// extension and `--accept-command`, where a change in the query string is a change too.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedCommand {
    /// The command shown.
    pub command: String,
    /// The archive URL shown.
    pub archive_url: String,
    /// A change in the query string is a change too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_counts: Option<bool>,
}

/// One `headers`/`headersHelper` declaration, with what the policy needs to know about it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderLayer {
    /// Where it is set.
    pub place: HeaderPlace,
    /// Where the declaration lives.
    pub origin: HeaderOrigin,
    /// The fixed headers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, String>>,
    /// The command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers_helper: Option<String>,
    /// The `CLAUDE_CODE_*` variables Claude Code sets for the command (`urlSourceLayer`, `entryLayer`).
    pub variables: IndexMap<String, String>,
    /// A `url` source: the marketplace URL the command is declared for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace_url: Option<String>,
    /// An entry: what the user is doing, and the command and archive URL they accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<PluginOperation>,
    /// An entry: the archive URL its command runs for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_url: Option<String>,
    /// The command and archive URL the user accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<AcceptedCommand>,
}

/// What the host knows when it asks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadersSetting {
    /// The host's platform.
    pub platform: PathPlatform,
    /// The user accepted the workspace trust dialog for the project folder itself.
    pub folder_trusted: bool,
    /// The user approved the delivered server-managed settings in the security approval dialog.
    pub settings_approved: bool,
    /// `disableCommandPluginSources`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_command_plugin_sources: Option<bool>,
    /// `allowManagedHooksOnly`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_managed_hooks_only: Option<bool>,
    /// Managed settings themselves declare the marketplace the command belongs to.
    pub managed_marketplace: bool,
}
