//! Port of `src/marketplace/version-resolution.ts`, switched.
//!
//! The version of an installed plugin as Claude Code computes it (plugins/loading.md, "How Claude
//! Code computes the version"): for every source type except `command` the manifest's `version`,
//! then the entry's, then what the source gives; for a `command` source always what the command
//! produced. The version names the cache directory and decides whether an update changes anything,
//! so it is a decision the host reads, never recomputes. Pure over what the host reports; only the
//! hash of a `git-subdir` path goes through the `Sha256` port.

use serde::{Deserialize, Serialize};

use super::version_hash::{short_hash_12, subdirectory_path_hash};
use crate::format::marketplace::types::PluginSourceKind;
use crate::js::trim;
use crate::ports::marketplace::Sha256;
use crate::registry::Port;

/// The version of a source that gives nothing to version it by (loading.md: "`unknown`").
pub const UNKNOWN_VERSION: &str = "unknown";

/// Where a version came from; `claude-ai` is the version a marketplace hosted on claude.ai records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionOrigin {
    /// The manifest's `version`.
    #[serde(rename = "manifest")]
    Manifest,
    /// The entry's `version`.
    #[serde(rename = "entry")]
    Entry,
    /// A commit.
    #[serde(rename = "sha")]
    Sha,
    /// An archive's digest.
    #[serde(rename = "sha256")]
    Sha256,
    /// The hash of what a `command` source produced.
    #[serde(rename = "command-hash")]
    CommandHash,
    /// Nothing to version it by.
    #[serde(rename = "unknown")]
    Unknown,
    /// Recorded by claude.ai.
    #[serde(rename = "claude-ai")]
    ClaudeAi,
}

/// A version and where it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PluginVersion {
    /// The version.
    pub version: String,
    /// Where it came from.
    pub origin: VersionOrigin,
}

/// What the host knows about a plugin when the version is decided, with the source of the plugin
/// (`VersionFacts & VersionSource`): a `git-subdir` source always has its `subdir`, which its
/// version hashes; every fact the host lacks is left out.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInput {
    /// The kind of source.
    pub source_kind: PluginSourceKind,
    /// `path` of the `git-subdir` source.
    #[serde(default)]
    pub subdir: Option<String>,
    /// `version` in the plugin's manifest (the merged manifest when the entry supplies it, `mergeEntryWithManifest`).
    #[serde(default)]
    pub manifest_version: Option<String>,
    /// `version` of the marketplace entry.
    #[serde(default)]
    pub entry_version: Option<String>,
    /// The commit of the cloned repository or, for a relative path, of the installed directory.
    #[serde(default)]
    pub commit_sha: Option<String>,
    /// The `sha256` pin of an `archive` entry.
    #[serde(default)]
    pub archive_sha256: Option<String>,
    /// The digest of the downloaded archive, used when the entry has no pin.
    #[serde(default)]
    pub archive_digest: Option<String>,
    /// The hash of what a `command` source produced (`copyModeHash`, `linkModeHash`).
    #[serde(default)]
    pub command_hash: Option<String>,
    /// A relative-path plugin whose directory or whose marketplace is a Git repository: either one
    /// gives a commit to version it by; neither gives `unknown` ("Local directory, when neither the
    /// plugin directory nor its marketplace is a git repository").
    pub in_git_repository: bool,
    /// The only repository the host found encloses the install path (a git-managed `~/.claude`): its commit is not a version.
    pub enclosing_repo_ignored: bool,
    /// The version a marketplace hosted on claude.ai records for the plugin.
    #[serde(default)]
    pub recorded_by_claude_ai: Option<String>,
}

/// Whether a version can name one folder: "The version also names the plugin's cache directory"
/// (loading.md), so a version that is empty, `.`, `..` or holds a separator or a NUL would leave
/// `cache/<marketplace>/<plugin>/`. The docs do not restrict the characters; any other text is kept
/// verbatim (spec/judgments.md).
#[must_use]
pub fn is_version_segment(version: &str) -> bool {
    !version.is_empty() && version != "." && version != ".." && !version.contains(['/', '\\', '\0'])
}

fn unknown() -> PluginVersion {
    PluginVersion {
        version: UNKNOWN_VERSION.to_owned(),
        origin: VersionOrigin::Unknown,
    }
}

/// A version field that says something: an empty or blank string is no version (spec/judgments.md).
fn stated_version(value: Option<&str>) -> Option<&str> {
    value.filter(|text| !trim(text).is_empty())
}

/// A stated version that also names one folder; one that does not is skipped, as if unset (spec/judgments.md).
#[must_use]
pub fn usable_version(value: Option<&str>) -> Option<String> {
    stated_version(value)
        .filter(|text| is_version_segment(text))
        .map(str::to_owned)
}

fn from_commit(input: &VersionInput) -> PluginVersion {
    stated_version(input.commit_sha.as_deref()).map_or_else(unknown, |commit| PluginVersion {
        version: short_hash_12(commit),
        origin: VersionOrigin::Sha,
    })
}

async fn from_subdirectory(input: &VersionInput, sha256: &dyn Sha256) -> PluginVersion {
    let commit = from_commit(input);
    let Some(subdir) = input
        .subdir
        .as_deref()
        .filter(|_| commit.origin != VersionOrigin::Unknown)
    else {
        return unknown();
    };
    let path_hash = subdirectory_path_hash(subdir, sha256).await;
    PluginVersion {
        version: format!("{}-{path_hash}", commit.version),
        origin: VersionOrigin::Sha,
    }
}

fn from_archive(input: &VersionInput) -> PluginVersion {
    let digest = stated_version(input.archive_sha256.as_deref())
        .or_else(|| stated_version(input.archive_digest.as_deref()));
    digest.map_or_else(unknown, |digest| PluginVersion {
        version: short_hash_12(digest),
        origin: VersionOrigin::Sha256,
    })
}

/// A relative path is versioned by a commit only inside a Git-hosted marketplace and outside an enclosing repository.
fn from_relative(input: &VersionInput) -> PluginVersion {
    if input.in_git_repository && !input.enclosing_repo_ignored {
        from_commit(input)
    } else {
        unknown()
    }
}

/// What each source type gives when neither the manifest nor the entry has a `version`.
async fn from_source(
    kind: PluginSourceKind,
    input: &VersionInput,
    sha256: &dyn Sha256,
) -> PluginVersion {
    match kind {
        PluginSourceKind::Relative => from_relative(input),
        PluginSourceKind::Github | PluginSourceKind::Url => from_commit(input),
        PluginSourceKind::GitSubdir => from_subdirectory(input, sha256).await,
        PluginSourceKind::Npm | PluginSourceKind::Command => unknown(),
        PluginSourceKind::Archive => from_archive(input),
    }
}

/// The manifest's `version`, else the entry's (loading.md, "How Claude Code computes the version", steps 1 and 2).
fn declared(input: &VersionInput) -> Option<PluginVersion> {
    if let Some(manifest) = usable_version(input.manifest_version.as_deref()) {
        return Some(PluginVersion {
            version: manifest,
            origin: VersionOrigin::Manifest,
        });
    }
    usable_version(input.entry_version.as_deref()).map(|entry| PluginVersion {
        version: entry,
        origin: VersionOrigin::Entry,
    })
}

/// "A 12-character hash on its own, or `<manifest version>-<hash>` when the manifest sets one. The
/// marketplace entry's `version` is ignored for command sources." A command source whose hash the
/// host did not supply is `unknown`, never the entry's version (spec/judgments.md).
fn from_command(input: &VersionInput) -> PluginVersion {
    let Some(hash) = stated_version(input.command_hash.as_deref()) else {
        return unknown();
    };
    let manifest = usable_version(input.manifest_version.as_deref());
    let short = short_hash_12(hash);
    PluginVersion {
        version: manifest.map_or_else(|| short.clone(), |manifest| format!("{manifest}-{short}")),
        origin: VersionOrigin::CommandHash,
    }
}

/// One rule per source type: the declared version, then what the source gives; a `command` source its hash alone.
async fn version_rule(input: &VersionInput, sha256: &dyn Sha256) -> PluginVersion {
    if input.source_kind == PluginSourceKind::Command {
        return from_command(input);
    }
    match declared(input) {
        Some(version) => version,
        None => from_source(input.source_kind, input, sha256).await,
    }
}

/// The version of a plugin. A plugin of a marketplace hosted on claude.ai has "the version claude.ai
/// records for the plugin ... and the manifest's `version` isn't read".
pub async fn resolve_plugin_version(input: &VersionInput, sha256: &dyn Sha256) -> PluginVersion {
    if let Some(recorded) = usable_version(input.recorded_by_claude_ai.as_deref()) {
        return PluginVersion {
            version: recorded,
            origin: VersionOrigin::ClaudeAi,
        };
    }
    let computed = version_rule(input, sha256).await;
    if is_version_segment(&computed.version) {
        computed
    } else {
        unknown()
    }
}

crate::registry::exports! {
    "marketplace/version-resolution" {
        "UNKNOWN_VERSION" => ok || UNKNOWN_VERSION;
        "isVersionSegment" => ok |version: String| is_version_segment(&version);
        "usableVersion" => ok |value: Option<String>| usable_version(value.as_deref());
        "resolvePluginVersion" => async ok |input: VersionInput, sha256: Port|
            resolve_plugin_version(&input, &sha256).await;
    }
}
