//! Port of `src/cli/flags/plugins.ts`, switched.
//!
//! `--plugin-dir` and `--plugin-url` (cli-reference.md, "CLI flags"; plugins/cli-reference.md,
//! "Flags that load a plugin for one session"): plugins for this session only, of the `inline`
//! origin (`<name>@inline`, `--json` scope `session`). The kit maps the flags to requests; the host
//! reads the folder (a folder that is no plugin holds plugins: `readPluginDirectory`), fetches the
//! archive and names each plugin from its manifest. Which same-named copy loads is the install
//! stream's (`copyThatLoads`).

use super::sideload::sideload_refusal;
use super::types::{
    Contribution, FlagContext, SessionPluginOrigin, SessionPluginRequest, SessionPluginSource,
    refuse,
};
use super::values::words_of;
use crate::format::path_text::without_trailing_slashes;
use crate::loading::plugin_folder::PluginDirectoryReading;
use crate::marketplace::semver::satisfies_range;

/// `/\.zip$/i`.
fn is_archive(path: &str) -> bool {
    path.len()
        .checked_sub(4)
        .and_then(|start| path.get(start..))
        .is_some_and(|end| end.eq_ignore_ascii_case(".zip"))
}

fn request_of(source: SessionPluginSource, location: &str) -> SessionPluginRequest {
    SessionPluginRequest {
        source,
        location: location.to_owned(),
        origin: SessionPluginOrigin::Inline,
    }
}

/// "Each flag takes one path. Repeat the flag for more paths": a path is never split, even with a
/// space in it.
#[must_use]
pub fn plugin_dir(paths: &[String], context: &FlagContext) -> Contribution {
    if let Some(refusal) = sideload_refusal("--plugin-dir", &context.facts) {
        return refuse(vec![refusal]);
    }
    Contribution {
        session_plugins: Some(
            paths
                .iter()
                .map(|path| {
                    let source = if is_archive(path) {
                        SessionPluginSource::Archive
                    } else {
                        SessionPluginSource::Directory
                    };
                    request_of(source, path)
                })
                .collect(),
        ),
        ..Contribution::default()
    }
}

/// "Repeat the flag for multiple plugins, or pass space-separated URLs in a single quoted value."
#[must_use]
pub fn plugin_url(values: &[String], context: &FlagContext) -> Contribution {
    if let Some(refusal) = sideload_refusal("--plugin-url", &context.facts) {
        return refuse(vec![refusal]);
    }
    let urls = values.iter().flat_map(|value| words_of(value));
    Contribution {
        session_plugins: Some(
            urls.map(|url| request_of(SessionPluginSource::Url, &url))
                .collect(),
        ),
        ..Contribution::default()
    }
}

const PARENT_FOLDERS: &str = ">=2.1.265";

/// A directory is itself unless it holds plugins; a folder of plugins needs Claude Code 2.1.265
/// (plugins/create.md: "Loading a folder of plugins requires Claude Code v2.1.265 or later"), and an
/// older host loads nothing from it. `host_version` absent means a current version.
fn directory_folders(
    location: &str,
    reading: &PluginDirectoryReading,
    host_version: Option<&str>,
) -> Vec<String> {
    if !reading.is_folder {
        return vec![location.to_owned()];
    }
    if host_version.is_some_and(|version| !satisfies_range(version, PARENT_FOLDERS)) {
        return Vec::new();
    }
    reading
        .plugins
        .iter()
        .map(|name| format!("{}/{name}", without_trailing_slashes(location)))
        .collect()
}

/// The folders to load for one request, given what `readPluginDirectory` read in the directory:
/// an archive or a URL is one plugin; a directory is itself when it is a plugin, else each
/// immediate subfolder that holds a `.claude-plugin/plugin.json`.
#[must_use]
pub fn plugin_folders_of(
    request: &SessionPluginRequest,
    reading: &PluginDirectoryReading,
    host_version: Option<&str>,
) -> Vec<String> {
    match request.source {
        SessionPluginSource::Directory => {
            directory_folders(&request.location, reading, host_version)
        }
        SessionPluginSource::Archive | SessionPluginSource::Url => vec![request.location.clone()],
    }
}

crate::registry::exports! {
    "cli/flags/plugins" {
        "pluginDir" => ok |paths: Vec<String>, context: FlagContext| plugin_dir(&paths, &context);
        "pluginUrl" => ok |values: Vec<String>, context: FlagContext| plugin_url(&values, &context);
        "pluginFoldersOf" => ok |request: SessionPluginRequest, reading: PluginDirectoryReading, host_version: Option<String>|
            plugin_folders_of(&request, &reading, host_version.as_deref());
    }
}
