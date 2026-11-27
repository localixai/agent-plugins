//! The core's counterpart of `src/marketplace/types.ts` at sha256:0b08d4511a4b., which stays TypeScript: it declares
//! types only, so it has nothing to run (docs/RUST.md, "What stays TypeScript"). Only the types
//! the ported modules of this folder use are declared here; the rest follow with the modules that
//! use them.
//!
//! Contracts of the marketplace module (layer 3): the plans a host executes and the small values
//! several pieces share (docs/API.md, "Marketplace"). Planning is pure; these are what it returns.
//! Pages: plugins/marketplace-reference.md ("Plugin sources", "Marketplace sources"),
//! plugins/host-marketplace.md ("Stay within the download limits", "Rename or remove a plugin"),
//! plugins/dependencies.md.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::source_safety::NpmSpec;
use super::submodules::MarketplaceSource;
use crate::format::path_text::PathPlatform;
use crate::js::JsValue;
use crate::ports::marketplace::GitRepository;

/// Header names are lower case, as on the ports (`HttpRequest`).
pub type HeaderMap = IndexMap<String, String>;

/// The shell a `command` source runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandShell {
    /// `sh -c`.
    Sh,
    /// `cmd.exe /c`.
    Cmd,
}

/// Where a `command` source runs: the user's home directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandCwd {
    /// The user's home directory.
    Home,
}

/// How a `command` source's directory is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandMode {
    /// Copied into the cache.
    Copy,
    /// Linked from the cache.
    Link,
}

/// A `command` source's fetch plan (`kind: "command"`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandFetchPlan {
    /// The command line.
    pub command: String,
    /// The shell that runs it.
    pub shell: CommandShell,
    /// Where it runs.
    pub cwd: CommandCwd,
    /// How long it may run.
    pub timeout_ms: f64,
    /// How its directory is installed.
    pub mode: CommandMode,
}

/// Which of the marketplace repository's git submodules a local plugin needs (marketplace-
/// reference.md, "Git submodules in the marketplace repository"): `none` for a sparse checkout or a
/// marketplace that was not cloned, `plugin-subtree` for a plugin in a subdirectory (the submodules
/// it is in, holds or links into), `whole-repository` for a plugin at the root (all of them or none).
/// `selectSubmodules` applies the rule to the repository's files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmoduleRule {
    /// `none`.
    #[serde(rename = "none")]
    None,
    /// `plugin-subtree`.
    #[serde(rename = "plugin-subtree")]
    PluginSubtree,
    /// `whole-repository`.
    #[serde(rename = "whole-repository")]
    WholeRepository,
}

/// A dependency with its defaults applied (plugins/dependencies.md): the marketplace is the
/// declaring one when omitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedDependency {
    /// The plugin's name.
    pub name: String,
    /// Its marketplace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace: Option<String>,
    /// Its node-semver range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
}

/// Where a plugin name stands in a marketplace with a `renames` map (`RenameStatus`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum RenameStatus {
    /// It is listed.
    Current,
    /// It loads under another name.
    Renamed {
        /// The new name.
        to: String,
    },
    /// It is gone.
    Removed,
    /// Claude Code has no answer.
    Unknown,
}

/// The redirect rule of a downloaded `marketplace.json`: to a different origin, `https://` only,
/// never a blocked host, no configured headers sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceRedirects {
    /// `https-only`.
    pub cross_origin: String,
    /// `refused`.
    pub blocked_hosts: String,
    /// `none`.
    pub headers_across_origins: String,
}

/// How a marketplace file is fetched (`MarketplaceFetchPlan`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum MarketplaceFetchPlan {
    /// A clone.
    Git {
        /// The repository.
        repo: GitRepository,
        /// A branch or tag.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
        /// The file in it.
        path: String,
        /// The folders to fetch.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sparse_paths: Option<Vec<String>>,
        /// As for a plugin's clone: LFS content is never downloaded (`pointer-files`).
        lfs: String,
    },
    /// A download.
    HttpFile {
        /// The URL.
        url: String,
        /// 5 MiB.
        max_bytes: f64,
        /// 10 seconds.
        timeout_ms: f64,
        /// The redirect rule.
        redirects: MarketplaceRedirects,
        /// The headers it sends.
        headers: HeaderMap,
        /// The helper that makes more.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        helper: Option<String>,
    },
    /// A file read.
    ReadFile {
        /// The file.
        path: String,
        /// The marketplace root.
        root: String,
    },
    /// A folder read.
    ReadDirectory {
        /// The folder.
        root: String,
    },
    /// The entries written in the settings, as given.
    Inline {
        /// The entries.
        entries: Vec<JsValue>,
    },
}

/// The documented limits of an `archive` download and its extraction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveLimits {
    /// The zip's size.
    pub max_download_bytes: f64,
    /// How long the download may take.
    pub timeout_ms: f64,
    /// How many redirects it follows.
    pub max_redirects: f64,
    /// How many files and directories.
    pub max_entries: f64,
    /// One file's size uncompressed.
    pub max_file_bytes: f64,
    /// The whole content uncompressed.
    pub max_uncompressed_bytes: f64,
    /// Uncompressed content may be at most this many times the size of the zip.
    pub max_compression_ratio: f64,
}

/// An `archive` source's fetch plan (`kind: "archive"`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveFetchPlan {
    /// The zip's URL.
    pub url: String,
    /// The digest it must have.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Its limits.
    pub limits: ArchiveLimits,
    /// The headers the download sends.
    pub headers: HeaderMap,
    /// Where the plugin root may sit in the zip.
    #[serde(default)]
    pub root_depth: RootDepth,
}

/// Where an archive's plugin root may sit: at the top or one folder down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RootDepth {
    /// `top-or-one-down`.
    #[default]
    #[serde(rename = "top-or-one-down")]
    TopOrOneDown,
}

/// What a clone ends on: `sha` wins over `ref`, which wins over the default branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Checkout {
    /// `sha`.
    Sha,
    /// `ref`.
    Ref,
    /// `default-branch`.
    DefaultBranch,
}

/// A git source's fetch plan (`kind: "git"`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFetchPlan {
    /// The repository.
    pub repo: GitRepository,
    /// A branch or tag.
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// A commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// What the clone ends on: `sha` wins over `ref`, which wins over the default branch.
    pub checkout: Checkout,
    /// The folder checked out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<String>,
    /// A sparse checkout.
    pub sparse: bool,
    /// A partial clone.
    pub partial_clone: bool,
    /// "The clone never downloads LFS content, so LFS-tracked files arrive as pointer files"
    /// (host-marketplace.md, "Keep plugin files out of Git LFS"): a host runs the clone without
    /// fetching LFS objects.
    pub lfs: String,
}

/// An npm source's fetch plan (`kind: "npm"`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpmFetchPlan {
    /// The package and version, or a tarball link.
    pub spec: NpmSpec,
    /// The registry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// Always `false`.
    pub run_install_scripts: bool,
    /// Always `false`.
    pub install_dependencies: bool,
}

/// How a plugin source is fetched, as data: the kit plans it, the host (or an engine over ports)
/// runs it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum FetchPlan {
    /// Files already in the marketplace.
    Local {
        /// The plugin's path from the marketplace root.
        #[serde(rename = "fromMarketplaceRoot")]
        from_marketplace_root: String,
        /// The submodules it needs.
        submodules: SubmoduleRule,
    },
    /// A clone.
    Git(GitFetchPlan),
    /// An npm package.
    Npm(NpmFetchPlan),
    /// A zip download.
    Archive(ArchiveFetchPlan),
    /// A command that puts the plugin in place.
    Command(CommandFetchPlan),
}

/// What planning a plugin fetch needs to know about the host and the marketplace it reads.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchPlanContext {
    /// How the marketplace was added.
    #[serde(default)]
    pub marketplace_origin: Option<MarketplaceSource>,
    /// The `<plugin>@<marketplace>` being installed, named by a refusal (`Cannot install <id>: …`).
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// The registry the user's own npm configuration names; the only `http://` or non-default one
    /// allowed.
    #[serde(default)]
    pub default_npm_registry: Option<String>,
    /// The session's folder.
    #[serde(default)]
    pub session_cwd: Option<String>,
}
