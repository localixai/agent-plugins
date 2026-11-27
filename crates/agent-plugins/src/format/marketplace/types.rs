//! Port of `src/format/marketplace/types.ts`, switched.
//!
//! The contracts of the marketplace stream (docs/API.md, "Marketplace"): the shapes every later
//! piece of it reads and writes, decided once so no two of them declare a copy. Types and the two
//! closed lists of source kinds; nothing here runs.
//! Pages: plugins/marketplace-reference.md ("Top-level fields", "Plugin entries", "Plugin sources",
//! "Marketplace sources").

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::renames::Renames;
use crate::format::issues::PluginIssue;
use crate::js::JsValue;

pub use crate::protocol::settings_keys::MARKETPLACE_SOURCE_KINDS;

/// The seven ways a plugin entry's `source` can name where the plugin lives: a relative path
/// string, or an object whose `source` field is one of the other six (marketplace-reference.md,
/// "Plugin sources"). `command` is the kit's name for the command source; the SDK has no type for
/// any of them, they are `marketplace.json` shapes.
pub const PLUGIN_SOURCE_KINDS: [PluginSourceKind; 7] = [
    PluginSourceKind::Relative,
    PluginSourceKind::Github,
    PluginSourceKind::Url,
    PluginSourceKind::GitSubdir,
    PluginSourceKind::Npm,
    PluginSourceKind::Archive,
    PluginSourceKind::Command,
];

/// One of `PLUGIN_SOURCE_KINDS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginSourceKind {
    /// `relative`
    #[serde(rename = "relative")]
    Relative,
    /// `github`
    #[serde(rename = "github")]
    Github,
    /// `url`
    #[serde(rename = "url")]
    Url,
    /// `git-subdir`
    #[serde(rename = "git-subdir")]
    GitSubdir,
    /// `npm`
    #[serde(rename = "npm")]
    Npm,
    /// `archive`
    #[serde(rename = "archive")]
    Archive,
    /// `command`
    #[serde(rename = "command")]
    Command,
}

/// How a `command` source brings its plugin in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandMode {
    /// A copy.
    Copy,
    /// A link.
    Link,
}

/// A parsed plugin source. Registries are typed over `PluginSourceKind`, never a `switch`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum PluginSource {
    /// `path` is root-relative with the leading `./` removed and `'.'` for the marketplace root;
    /// `bare` says the entry wrote a bare name that resolves under `metadata.pluginRoot`.
    Relative {
        /// The path.
        path: String,
        /// A bare name.
        bare: bool,
    },
    /// A GitHub repository.
    Github {
        /// `owner/name`.
        repo: String,
        /// A branch or tag.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
        /// A commit.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha: Option<String>,
    },
    /// A git URL.
    Url {
        /// The URL.
        url: String,
        /// A branch or tag.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
        /// A commit.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha: Option<String>,
    },
    /// A folder of a git repository.
    GitSubdir {
        /// The URL.
        url: String,
        /// The folder.
        path: String,
        /// A branch or tag.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
        /// A commit.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha: Option<String>,
    },
    /// An npm package.
    Npm {
        /// The package.
        package: String,
        /// Its version.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        /// Its registry.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
    },
    /// An archive.
    Archive {
        /// The URL.
        url: String,
        /// Its digest.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha256: Option<String>,
    },
    /// A command that puts the plugin in place.
    Command {
        /// The command.
        command: String,
        /// How long it may run.
        timeout_seconds: f64,
        /// Copy or link.
        mode: CommandMode,
    },
}

impl PluginSource {
    /// The kind of the source.
    #[must_use]
    pub const fn kind(&self) -> PluginSourceKind {
        match self {
            Self::Relative { .. } => PluginSourceKind::Relative,
            Self::Github { .. } => PluginSourceKind::Github,
            Self::Url { .. } => PluginSourceKind::Url,
            Self::GitSubdir { .. } => PluginSourceKind::GitSubdir,
            Self::Npm { .. } => PluginSourceKind::Npm,
            Self::Archive { .. } => PluginSourceKind::Archive,
            Self::Command { .. } => PluginSourceKind::Command,
        }
    }
}

/// One of the ten forms a marketplace source takes: in `extraKnownMarketplaces`, in `marketplace
/// add` input, and, for the last three, only in the policy lists (marketplace-reference.md,
/// "Marketplace sources", "Source values valid only in policy lists"). One list with the
/// settings.
pub type MarketplaceSourceKind = String;

/// The owner of a marketplace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    /// The owner's name.
    pub name: String,
    /// Their address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Their page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A manifest dependency of a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDep {
    /// The file.
    pub file: String,
    /// What it must hold.
    pub pattern: String,
}

/// `relevance.signals` of an entry (plugins/relevance.md, "Field reference").
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelevanceSignals {
    /// Working directories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<Vec<String>>,
    /// Commands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli: Option<Vec<String>>,
    /// Hosts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosts: Option<Vec<String>>,
    /// Files read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_read: Option<Vec<String>>,
    /// Manifest dependencies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_deps: Option<Vec<ManifestDep>>,
}

/// `relevance` of an entry (plugins/relevance.md, "Field reference").
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relevance {
    /// The topic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    /// The signals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signals: Option<RelevanceSignals>,
}

/// A marketplace entry. The plugin.json fields an entry may carry beside its own
/// (marketplace-reference.md, "Plugin entries": "an entry also accepts every `plugin.json`
/// field"): with no `plugin.json` they all apply; the strict-mode table is about the first six
/// (`entry_merge`). A `PluginManifest` is a `JsValue` until its module is in the core.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceEntry {
    /// The plugin's name.
    pub name: String,
    /// Where it lives.
    pub source: PluginSource,
    /// Defaults to `true`; the entry only completes `plugin.json` (marketplace-reference.md,
    /// "Strict mode"). Its place is the one the reader of entries builds it in.
    pub strict: bool,
    /// What it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Its category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Its tags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Where it is relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relevance: Option<Relevance>,
    /// What it depends on (`PluginDependency`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<JsValue>>,
    /// Whether it is on once installed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_enabled: Option<bool>,
    /// How it is shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Anything else the entry carries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<IndexMap<String, JsValue>>,
    /// Archive downloads only (host-marketplace.md, "Authenticate archive downloads").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, String>>,
    /// The helper that makes them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers_helper: Option<String>,
    /// `Partial<Pick<PluginManifest, ComponentKey>>`: `commands`, `agents`, `skills`, `hooks`,
    /// `outputStyles`, `themes`, `mcpServers`, `lspServers`, `userConfig`, `channels`, `settings`,
    /// `workflows`, `monitors`, `experimental`, `types`.
    pub components: IndexMap<String, JsValue>,
    /// `Partial<Pick<PluginManifest, DisplayKey>>`: `author`, `homepage`, `repository`, `license`,
    /// `keywords`.
    pub display: IndexMap<String, JsValue>,
}

/// A marketplace source (`MarketplaceSourceKind`); the `path` of git sources defaults to
/// `.claude-plugin/marketplace.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum MarketplaceSource {
    /// A file at a URL.
    #[serde(rename = "url")]
    Url {
        /// The URL.
        url: String,
        /// Its headers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        headers: Option<IndexMap<String, String>>,
        /// The helper that makes them.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        headers_helper: Option<String>,
    },
    /// A GitHub repository.
    #[serde(rename = "github")]
    Github {
        /// `owner/name`.
        repo: String,
        /// A branch or tag.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
        /// The file.
        path: String,
        /// The folders to fetch.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sparse_paths: Option<Vec<String>>,
        /// Accepted and has no effect (marketplace-reference.md, "Fields by type"); kept so a
        /// source round-trips.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skip_lfs: Option<bool>,
    },
    /// Any git repository.
    #[serde(rename = "git")]
    Git {
        /// The URL.
        url: String,
        /// A branch or tag.
        #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
        /// The file.
        path: String,
        /// The folders to fetch.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sparse_paths: Option<Vec<String>>,
        /// Accepted and has no effect (marketplace-reference.md, "Fields by type"); kept so a
        /// source round-trips.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skip_lfs: Option<bool>,
    },
    /// An npm package.
    #[serde(rename = "npm")]
    Npm {
        /// The package.
        package: String,
        /// In the SDK's `source` shape; the marketplace-sources table lists only `package`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        /// Its registry.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
    },
    /// A file.
    #[serde(rename = "file")]
    File {
        /// Where.
        path: String,
    },
    /// A folder.
    #[serde(rename = "directory")]
    Directory {
        /// Where.
        path: String,
    },
    /// A marketplace written in the settings.
    #[serde(rename = "settings")]
    Settings {
        /// Its name.
        name: String,
        /// Its entries.
        plugins: Vec<MarketplaceEntry>,
        /// Its owner.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner: Option<Owner>,
    },
    /// The skills folder.
    #[serde(rename = "skills-dir")]
    SkillsDir {},
    /// A host pattern.
    #[serde(rename = "hostPattern")]
    HostPattern {
        /// The pattern.
        host_pattern: String,
    },
    /// A path pattern.
    #[serde(rename = "pathPattern")]
    PathPattern {
        /// The pattern.
        path_pattern: String,
    },
}

/// What `extraKnownMarketplaces` does with a source when the marketplace is added: the docs name
/// two that parse and then fail to load (marketplace-reference.md, "Source values valid only in
/// policy lists" and the `npm` row of "Marketplace sources").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceLoadStatus {
    /// It loads.
    #[serde(rename = "loads")]
    Loads,
    /// An npm source.
    #[serde(rename = "fails: NPM marketplace sources not yet implemented")]
    FailsNpm,
    /// A policy-only source.
    #[serde(rename = "fails: Unsupported marketplace source type")]
    FailsPolicyOnly,
    /// An owner wildcard.
    #[serde(rename = "fails: The owner wildcard names no single repository to clone")]
    FailsOwnerWildcard,
}

/// What parsing a marketplace source gave.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceParseResult {
    /// The source, when it parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<MarketplaceSource>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
    /// What adding it would do.
    pub load_status: MarketplaceLoadStatus,
}

/// `metadata` of a marketplace.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceMetadata {
    /// What it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// What a bare name resolves under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_root: Option<String>,
    /// The keys Claude Code does not read, kept as they were for the host (warned about; judgment,
    /// spec/judgments.md).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<IndexMap<String, JsValue>>,
}

/// A marketplace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marketplace {
    /// Its name.
    pub name: String,
    /// Its owner.
    pub owner: Owner,
    /// Its entries.
    pub plugins: Vec<MarketplaceEntry>,
    /// What it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Its metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MarketplaceMetadata>,
    /// Whether a plugin that left the list is removed.
    pub force_remove_deleted_plugins: bool,
    /// The marketplaces a plugin may depend on.
    pub allow_cross_marketplace_dependencies_on: Vec<String>,
    /// Old plugin name to the new one, or `null` for a plugin that was removed.
    pub renames: Renames,
}

pub use super::plugin_source_fields::ReadMode;

/// What the host knows about the marketplace it hands to the reader.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceContext {
    /// Where the marketplace came from: decides what a relative-path source resolves against and
    /// the official-name exemption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<MarketplaceSource>,
    /// The marketplace is hosted on claude.ai, the only one whose name may start with `claudeai-`
    /// (marketplace-reference.md, "Reserved names").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_on_claude_ai: Option<bool>,
    /// Whose path rules apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<crate::format::path_text::PathPlatform>,
    /// The marketplace's own `name`: an official marketplace's entries are not checked against the
    /// reserved plugin names (oracle 2.1.319).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace_name: Option<String>,
    /// `plugin.json` of each relative-path entry by entry index, for the `plugins[i] plugin.json
    /// →` reports: a `ReadonlyMap`, as the wrapper tags it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_manifests: Option<crate::registry::Map<u32, JsValue>>,
}

/// What reading a marketplace gave.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceResult {
    /// The marketplace, when it read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace: Option<Marketplace>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

crate::registry::exports! {
    "format/marketplace/types" {
        "PLUGIN_SOURCE_KINDS" => ok || PLUGIN_SOURCE_KINDS;
    }
}
