//! Port of `src/marketplace/session-plugins.ts`, switched.
//!
//! The plugins a session loads from where they are, not from a marketplace (plugins/create.md,
//! "Load a plugin for one session": "Load a plugin from a directory or `.zip`", "Load a folder of
//! plugins", "Load a plugin from a URL"; plugins/cli-reference.md, "Flags that load a plugin for
//! one session"): what a `--plugin-dir` argument is, whether a directory is one plugin or a folder
//! of them, which subfolders load, what happens when one appears or goes away mid-session, and how
//! `--plugin-url` archives are fetched. Pure over `PluginFiles`; the archive download goes through
//! the same ports as an `archive` source.

use indexmap::IndexMap;
use serde::Serialize;

use super::archive_source::{ArchivePorts, ArchivePortsGiven, FetchedArchive, fetch_archive};
use super::command_source::WithSignal;
use super::fetch_plan::ARCHIVE_LIMITS;
use super::types::{ArchiveFetchPlan, RootDepth};
use crate::format::issues::PluginIssue;
use crate::js::is_whitespace;
use crate::ports::Signal;

/// What a `--plugin-dir` argument names: "a directory or a `.zip` archive of one".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginDirArgument {
    /// A directory.
    Directory,
    /// A `.zip` archive of one.
    Archive,
}

/// What a `--plugin-dir` argument names: "a directory or a `.zip` archive of one".
#[must_use]
pub fn plugin_dir_argument(path: &str) -> PluginDirArgument {
    // `/\.zip$/i`: without `u`, `i` folds ASCII letters alone.
    let zip = path
        .len()
        .checked_sub(4)
        .and_then(|start| path.get(start..))
        .is_some_and(|end| end.eq_ignore_ascii_case(".zip"));
    if zip {
        PluginDirArgument::Archive
    } else {
        PluginDirArgument::Directory
    }
}

/// `--plugin-url`: "Repeat the flag, or pass the URLs space-separated in one quoted argument."
#[must_use]
pub fn plugin_urls(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(is_whitespace))
        .filter(|url| !url.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether a plugin loads or unloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Change {
    /// It loads.
    Load,
    /// It unloads.
    Unload,
}

/// The first segment of every standard component location but the manifest's: what a plugin has
/// at its top level.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FolderChange {
    /// The plugin.
    pub plugin: String,
    /// Load or unload.
    pub change: Change,
    /// False when the change is held for `/reload-plugins`: it "would invalidate the prompt
    /// cache".
    pub applied: bool,
}

/// What an interactive session does when the folder's plugins change after startup: "A subfolder
/// you add loads as a new plugin once its manifest exists. When you remove a subfolder, its
/// plugin unloads." A change that would invalidate the prompt cache "is held instead", and the
/// message tells you to run `/reload-plugins`. Names that did not change give no change.
#[must_use]
pub fn folder_changes(
    loaded: &[String],
    now: &[String],
    invalidates_prompt_cache: bool,
) -> Vec<FolderChange> {
    let applied = !invalidates_prompt_cache;
    let added = now
        .iter()
        .filter(|name| !loaded.contains(name))
        .map(|plugin| FolderChange {
            plugin: plugin.clone(),
            change: Change::Load,
            applied,
        });
    let removed = loaded
        .iter()
        .filter(|name| !now.contains(name))
        .map(|plugin| FolderChange {
            plugin: plugin.clone(),
            change: Change::Unload,
            applied,
        });
    added.chain(removed).collect()
}

/// An archive that loaded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoadedArchive {
    /// Its URL.
    pub url: String,
    /// The verified zip.
    pub archive: FetchedArchive,
}

/// An archive that did not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ArchiveError {
    /// Its URL.
    pub url: String,
    /// Why.
    pub issue: PluginIssue,
}

/// What `--plugin-url` loaded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionArchives {
    /// The archives that loaded.
    pub loaded: Vec<LoadedArchive>,
    /// "A plugin load error that you can review in the `/plugin` manager's Errors tab."
    pub errors: Vec<ArchiveError>,
}

/// `--plugin-url`: the archives are downloaded at startup. "If Claude Code can't fetch the
/// archive, or the archive is invalid, it starts without the plugin and records a plugin load
/// error": one bad URL never stops the others, nor the session. Judgment (spec/judgments.md): the
/// download and extraction limits are those of an `archive` source, and no header is sent.
pub async fn fetch_session_archives(
    urls: &[String],
    ports: &ArchivePorts<'_>,
    signal: &Signal,
) -> SessionArchives {
    let mut archives = SessionArchives {
        loaded: Vec::new(),
        errors: Vec::new(),
    };
    for url in plugin_urls(urls) {
        let plan = ArchiveFetchPlan {
            url: url.clone(),
            sha256: None,
            limits: ARCHIVE_LIMITS,
            headers: IndexMap::new(),
            root_depth: RootDepth::TopOrOneDown,
        };
        match fetch_archive(&plan, ports, signal).await {
            Err(issue) => archives.errors.push(ArchiveError { url, issue }),
            Ok(archive) => archives.loaded.push(LoadedArchive { url, archive }),
        }
    }
    archives
}

crate::registry::exports! {
    "marketplace/session-plugins" {
        "pluginDirArgument" => ok |path: String| plugin_dir_argument(&path);
        "pluginUrls" => ok |values: Vec<String>| plugin_urls(&values);
        "folderChanges" => ok |loaded: Vec<String>, now: Vec<String>, invalidates_prompt_cache: bool|
            folder_changes(&loaded, &now, invalidates_prompt_cache);
        "fetchSessionArchives" => async ok |urls: Vec<String>, ports: ArchivePortsGiven, with: WithSignal|
            fetch_session_archives(&urls, &ports.ports(), &with.signal).await;
    }
}
