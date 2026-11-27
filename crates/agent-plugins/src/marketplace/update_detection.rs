//! Port of `src/marketplace/update-detection.ts`, switched.
//!
//! How Claude Code decides a plugin has an update (plugins/loading.md, "Versions and updates"):
//! `claude plugin update` and background auto-update "compute the version again and don't replace
//! the cached copy when it matches what `installed_plugins.json` records". A new copy is made only
//! when the computed version differs; the kit decides, the host copies. The docs say a new copy is made
//! "only when the plugin's computed version differs from the one they have" (host-marketplace.md) and
//! do not say it must be higher; comparing as strings is the kit's judgment (spec/judgments.md).

use serde::{Deserialize, Serialize};

use super::version_resolution::{VersionInput, resolve_plugin_version, usable_version};
use crate::format::marketplace::types::{CommandMode, MarketplaceSource, PluginSource};
use crate::ports::marketplace::Sha256;
use crate::registry::Port;

/// What the version comparison says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateOutcome {
    /// The same version.
    #[serde(rename = "unchanged")]
    Unchanged,
    /// Another version: a new copy.
    #[serde(rename = "new copy")]
    NewCopy,
}

/// What `claude plugin update` prints when the computed version is unchanged: "`<name> is already at
/// the latest version (<version>).`" (plugins/loading.md, "Versions and updates"; cli-reference.md).
#[must_use]
pub fn up_to_date_message(name: &str, version: &str) -> String {
    format!("{name} is already at the latest version ({version}).")
}

/// Whether `computed` is an update over what `installed_plugins.json` records for the plugin. A
/// plugin with no record has nothing cached, so it installs (judgment, spec/judgments.md).
#[must_use]
pub fn is_update(installed: Option<&str>, computed: &str) -> UpdateOutcome {
    if installed == Some(computed) {
        UpdateOutcome::Unchanged
    } else {
        UpdateOutcome::NewCopy
    }
}

/// The marketplace sources that are a local path: "a marketplace you added from a local path".
const fn local_marketplace(origin: &MarketplaceSource) -> bool {
    match origin {
        MarketplaceSource::File { .. } | MarketplaceSource::Directory { .. } => true,
        MarketplaceSource::Url { .. }
        | MarketplaceSource::Github { .. }
        | MarketplaceSource::Git { .. }
        | MarketplaceSource::Npm { .. }
        | MarketplaceSource::Settings { .. }
        | MarketplaceSource::SkillsDir {}
        | MarketplaceSource::HostPattern { .. }
        | MarketplaceSource::PathPattern { .. } => false,
    }
}

fn is_local_path(origin: Option<&MarketplaceSource>) -> bool {
    origin.is_some_and(local_marketplace)
}

/// A command source in `link` mode "loads the files in place"; in `copy` mode it is copied.
const fn linked_mode(mode: CommandMode) -> bool {
    match mode {
        CommandMode::Copy => false,
        CommandMode::Link => true,
    }
}

/// Per source type: does it load from where it is kept? (plugins/loading.md, "In-place and copied plugins".)
fn in_place(source: &PluginSource, origin: Option<&MarketplaceSource>) -> bool {
    match source {
        PluginSource::Relative { .. } => is_local_path(origin),
        PluginSource::Github { .. }
        | PluginSource::Url { .. }
        | PluginSource::GitSubdir { .. }
        | PluginSource::Npm { .. }
        | PluginSource::Archive { .. } => false,
        PluginSource::Command { mode, .. } => linked_mode(*mode),
    }
}

/// Whether a marketplace plugin is loaded in place rather than copied into the cache: a relative
/// path of a marketplace added from a local path, and a `command` source in `link` mode. Every other
/// marketplace plugin "is copied into `cache/<marketplace>/<plugin>/<version>/` at install". The
/// `--plugin-dir` and skills-directory plugins are in place too but come from no marketplace entry.
#[must_use]
pub fn loads_in_place(source: &PluginSource, origin: Option<&MarketplaceSource>) -> bool {
    in_place(source, origin)
}

/// What the update check is about: `source` and `origin` decide whether the plugin loads in place
/// (`loadsInPlace`); `version` is what the install computed its version from, given again with what
/// the host learns now (`resolvePluginVersion`, the one version rule: a plugin of a marketplace hosted
/// on claude.ai sets `recordedByClaudeAi`).
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateSubject {
    /// The entry's source.
    pub source: PluginSource,
    /// Its marketplace.
    #[serde(default)]
    pub origin: Option<MarketplaceSource>,
    /// What the version is computed from.
    pub version: VersionInput,
}

/// The outcome of the check (`UpdateCheck`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum UpdateCheck {
    /// The same version.
    #[serde(rename = "unchanged")]
    Unchanged,
    /// Another version: a new copy.
    #[serde(rename = "new copy")]
    NewCopy,
    /// Never compared.
    #[serde(rename = "loaded in place")]
    LoadedInPlace,
}

impl From<UpdateOutcome> for UpdateCheck {
    fn from(outcome: UpdateOutcome) -> Self {
        match outcome {
            UpdateOutcome::Unchanged => Self::Unchanged,
            UpdateOutcome::NewCopy => Self::NewCopy,
        }
    }
}

/// The outcome of the check and the version computed again, which a new copy is made at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpdateVerdict {
    /// The outcome.
    pub check: UpdateCheck,
    /// The version computed again.
    pub version: String,
}

/// The outcome of an update check for a plugin whose installed version is `installed`: a plugin that
/// loads in place is never compared, whatever its version string says.
pub async fn check_for_update(
    subject: &UpdateSubject,
    installed: Option<&str>,
    sha256: &dyn Sha256,
) -> UpdateVerdict {
    let version = resolve_plugin_version(&subject.version, sha256)
        .await
        .version;
    let check = if loads_in_place(&subject.source, subject.origin.as_ref()) {
        UpdateCheck::LoadedInPlace
    } else {
        is_update(installed, &version).into()
    };
    UpdateVerdict { check, version }
}

/// What the host knows of a plugin before it fetches anything: the entry's source and marketplace, and a claude.ai record.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePrecheck {
    /// The entry's source.
    pub source: PluginSource,
    /// Its marketplace.
    #[serde(default)]
    pub origin: Option<MarketplaceSource>,
    /// The version a marketplace hosted on claude.ai records for the plugin.
    #[serde(default)]
    pub recorded_by_claude_ai: Option<String>,
}

/// `fetch`: the version can come from what the fetch brings, so `checkForUpdate` decides after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PrecheckOutcome {
    /// The same version.
    #[serde(rename = "unchanged")]
    Unchanged,
    /// Another version: a new copy.
    #[serde(rename = "new copy")]
    NewCopy,
    /// Never compared.
    #[serde(rename = "loaded in place")]
    LoadedInPlace,
    /// Decided after the fetch.
    #[serde(rename = "fetch")]
    Fetch,
}

impl From<UpdateOutcome> for PrecheckOutcome {
    fn from(outcome: UpdateOutcome) -> Self {
        match outcome {
            UpdateOutcome::Unchanged => Self::Unchanged,
            UpdateOutcome::NewCopy => Self::NewCopy,
        }
    }
}

/// The update check a host can run before it fetches: a plugin loaded in place is never compared,
/// and a claude.ai record is the version ("the manifest's `version` isn't read"), compared by
/// `isUpdate`, the one comparison. Any other version may come from the manifest or the source, so it
/// is `fetch` (judgment, spec/judgments.md): a host never settles it by comparing versions itself.
#[must_use]
pub fn precheck_update(subject: &UpdatePrecheck, installed: Option<&str>) -> PrecheckOutcome {
    if loads_in_place(&subject.source, subject.origin.as_ref()) {
        return PrecheckOutcome::LoadedInPlace;
    }
    usable_version(subject.recorded_by_claude_ai.as_deref())
        .map_or(PrecheckOutcome::Fetch, |recorded| {
            is_update(installed, &recorded).into()
        })
}

crate::registry::exports! {
    "marketplace/update-detection" {
        "upToDateMessage" => ok |name: String, version: String| up_to_date_message(&name, &version);
        "isUpdate" => ok |installed: Option<String>, computed: String| is_update(installed.as_deref(), &computed);
        "loadsInPlace" => ok |source: PluginSource, origin: Option<MarketplaceSource>|
            loads_in_place(&source, origin.as_ref());
        "checkForUpdate" => async ok |subject: UpdateSubject, installed: Option<String>, sha256: Port|
            check_for_update(&subject, installed.as_deref(), &sha256).await;
        "precheckUpdate" => ok |subject: UpdatePrecheck, installed: Option<String>|
            precheck_update(&subject, installed.as_deref());
    }
}
