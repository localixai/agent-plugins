//! Port of `src/marketplace/install-mode.ts`, switched.
//!
//! How a plugin is installed: loaded where it lies, copied into the cache, or linked from the cache
//! entry (plugins/loading.md, "In-place and copied plugins"; plugins/marketplace-reference.md, "Copy
//! mode and link mode", "Output that fails the install or update"; plugins/host-marketplace.md,
//! "Edit plugins in place on a shared directory"). Pure: the origin and the source in, a decision or
//! the refusal out; the host copies, links or extracts.

use serde::{Deserialize, Serialize};

use super::command_mode::{LINKS_NEED_POSIX, link_on_windows_issue};
use super::command_output::{PrintedPath, contains_session, read_printed_path};
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::types::{CommandMode, MarketplaceSourceKind, PluginSource};
use crate::format::path_text::PathPlatform;

/// How a plugin is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallMode {
    /// Loaded where it lies.
    #[serde(rename = "in-place")]
    InPlace,
    /// Copied into the cache.
    #[serde(rename = "copy")]
    Copy,
    /// Linked from the cache entry.
    #[serde(rename = "link-in-cache")]
    LinkInCache,
}

/// Where a plugin came from, as far as how it is installed goes.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum InstallOrigin {
    /// `--plugin-dir` with a directory, or a folder of `CLAUDE_CODE_PLUGIN_DIRS`.
    SessionDirectory,
    /// `--plugin-dir` with a `.zip`, or `--plugin-url`.
    SessionArchive,
    /// A skills directory.
    SkillsDirectory,
    /// A marketplace's entry.
    Marketplace {
        /// How the marketplace was added; absent when the plugin has none (a copy of the entry is all there is).
        #[serde(default)]
        marketplace: Option<MarketplaceSourceKind>,
        /// The entry's source.
        source: PluginSource,
    },
}

/// What was decided.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the TypeScript `InstallDecision` carries these five facts, which the host reads by name"
)]
#[serde(rename_all = "camelCase")]
pub struct InstallDecision {
    /// How it is installed.
    pub mode: InstallMode,
    /// "A `--plugin-url` archive or a `--plugin-dir` `.zip` is extracted into a session temp directory first."
    pub extract_first: bool,
    /// Link mode: the cache entry holds links to the printed directory, which must stay where it is.
    pub linked: bool,
    /// The plugin has a folder in the cache (a copy, or the links of link mode); a plugin loaded in place has none.
    pub cached: bool,
    /// Whether Claude Code installs the plugin's Node.js packages (only into a copied version directory).
    pub dependency_install: bool,
    /// Whether an edit to the source reaches a session only through an update: a copy is a snapshot,
    /// a plugin loaded in place "take\[s\] effect at the next session start or `/reload-plugins`", and a
    /// link-mode plugin is loaded in place (judgment: the docs say "loads the files in place").
    pub edits_need_update: bool,
}

/// What the command printed and where the session started, for the checks that need them.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintedDirectory {
    /// The directory.
    pub directory: String,
    /// Where the session started.
    #[serde(default)]
    pub session_cwd: Option<String>,
}

/// What a decision is made from.
#[derive(Clone, Debug, Deserialize)]
pub struct ModeInput {
    /// Where the plugin came from.
    pub origin: InstallOrigin,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// For a `command` source: its printed directory.
    #[serde(default)]
    pub printed: Option<PrintedDirectory>,
}

/// What a mode means; the facts of a decision follow from it, one entry per mode.
const fn decision(mode: InstallMode, extract_first: bool) -> InstallDecision {
    match mode {
        InstallMode::InPlace => InstallDecision {
            mode,
            extract_first,
            linked: false,
            cached: false,
            dependency_install: false,
            edits_need_update: false,
        },
        // "Claude Code skips the Node.js package dependency install for a link-mode plugin"
        InstallMode::LinkInCache => InstallDecision {
            mode,
            extract_first,
            linked: true,
            cached: true,
            dependency_install: false,
            edits_need_update: false,
        },
        // "Claude Code installs the dependencies into the copied version directory"
        InstallMode::Copy => InstallDecision {
            mode,
            extract_first,
            linked: false,
            cached: true,
            dependency_install: true,
            edits_need_update: true,
        },
    }
}

/// "Relative-path plugins in a marketplace you added from a local path" load in place.
fn local_marketplace(kind: &str) -> bool {
    matches!(kind, "file" | "directory")
}

/// What each plugin source does; "Every other marketplace plugin" is copied.
fn mode_of_source(source: &PluginSource, local: bool) -> InstallMode {
    match source {
        PluginSource::Relative { .. } if local => InstallMode::InPlace,
        PluginSource::Command { mode, .. } => {
            // `mode`: "copy, the default, or link"; link "loads the files in place, through links in the cache entry".
            let links_need_posix = match mode {
                CommandMode::Copy => LINKS_NEED_POSIX.copy,
                CommandMode::Link => LINKS_NEED_POSIX.link,
            };
            if links_need_posix {
                InstallMode::LinkInCache
            } else {
                InstallMode::Copy
            }
        }
        PluginSource::Relative { .. }
        | PluginSource::Github { .. }
        | PluginSource::Url { .. }
        | PluginSource::GitSubdir { .. }
        | PluginSource::Npm { .. }
        | PluginSource::Archive { .. } => InstallMode::Copy,
    }
}

fn decision_of(origin: &InstallOrigin) -> InstallDecision {
    match origin {
        // "the directory loads in place and is never copied"
        InstallOrigin::SessionDirectory | InstallOrigin::SkillsDirectory => {
            decision(InstallMode::InPlace, false)
        }
        InstallOrigin::SessionArchive => decision(InstallMode::InPlace, true),
        InstallOrigin::Marketplace {
            marketplace,
            source,
        } => decision(
            mode_of_source(
                source,
                marketplace.as_deref().is_some_and(local_marketplace),
            ),
            false,
        ),
    }
}

/// The refusal when the printed directory is the session's own directory or one of its parents.
#[must_use]
pub fn session_directory_issue(printed: &str) -> PluginIssue {
    error(
        "source",
        &format!(
            "The printed directory \"{printed}\" is the directory Claude Code was started in, or one of its parents"
        ),
    )
}

/// What a printed directory fails on before its contents are read: a network path, the session's own directory.
fn printed_issue(printed: &PrintedDirectory, platform: PathPlatform) -> Option<PluginIssue> {
    if let PrintedPath::Problem { problem } = read_printed_path(&printed.directory, platform) {
        return Some(error("source.command", &problem));
    }
    contains_session(&printed.directory, printed.session_cwd.as_deref(), platform)
        .then(|| session_directory_issue(&printed.directory))
}

/// What `decideInstallMode` answers.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Decided {
    /// How the plugin is installed.
    Decision(InstallDecision),
    /// Why it may not be.
    Issue(PluginIssue),
}

/// How the plugin is installed, or why it may not be: "Claude Code refuses to install a link-mode
/// plugin on Windows", and a printed directory that is a UNC path on Windows or "the one Claude Code
/// was started in, or one of its parents" fails the install.
#[must_use]
pub fn decide_install_mode(input: &ModeInput) -> Decided {
    let decided = decision_of(&input.origin);
    if input.platform == PathPlatform::Windows && decided.linked {
        return Decided::Issue(link_on_windows_issue());
    }
    let refused = input
        .printed
        .as_ref()
        .and_then(|printed| printed_issue(printed, input.platform));
    refused.map_or(Decided::Decision(decided), Decided::Issue)
}

/// "Sessions started inside the directory": "a session started in the printed directory or anywhere
/// below it doesn't load" a link-mode plugin. Only link mode loads from the printed directory.
#[must_use]
pub fn linked_plugin_loads(
    decided: &InstallDecision,
    printed: &PrintedDirectory,
    platform: PathPlatform,
) -> bool {
    !decided.linked
        || !contains_session(&printed.directory, printed.session_cwd.as_deref(), platform)
}

crate::registry::exports! {
    "marketplace/install-mode" {
        "sessionDirectoryIssue" => ok |printed: String| session_directory_issue(&printed);
        "decideInstallMode" => ok |input: ModeInput| decide_install_mode(&input);
        "linkedPluginLoads" => ok |decided: InstallDecision, printed: PrintedDirectory, platform: PathPlatform|
            linked_plugin_loads(&decided, &printed, platform);
    }
}
