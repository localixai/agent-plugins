//! Port of `src/marketplace/node-deps-types.ts`, switched.
//!
//! The Node.js package dependencies of a plugin (plugins/loading.md, "Node.js package
//! dependencies": "When the dependency install runs", "Limits on the dependency install";
//! plugins/marketplace-reference.md, "Copy mode and link mode", "npm plugin source"): the plan the
//! facts of the plugin's `package.json` and lockfiles come to (install through `CommandRunner`, or
//! skip and why). Why an install did not happen, and what the user is told, is `node_deps_failure`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::install_mode::InstallMode;
use super::node_deps_lockfile::LockfileName;
use crate::format::issues::PluginIssue;
use crate::ports::command_runner::CommandSpec;

/// `bun` or `npm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    /// `bun`.
    Bun,
    /// `npm`.
    Npm,
}

/// Where the plugin's files lie, as far as the install goes (`InstallMode`, `InstallOrigin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DepsOrigin {
    /// Copied into a version directory.
    Copied,
    /// Loaded where it lies.
    InPlace,
    /// A session-only plugin, never an install mode.
    Inline,
    /// Linked from the cache entry.
    Linked,
}

/// The install mode of `decideInstallMode` as the origin the dependency plan takes.
#[must_use]
pub const fn deps_origin_of_mode(mode: InstallMode) -> DepsOrigin {
    match mode {
        InstallMode::Copy => DepsOrigin::Copied,
        InstallMode::InPlace => DepsOrigin::InPlace,
        InstallMode::LinkInCache => DepsOrigin::Linked,
    }
}

/// `DEPS_ORIGIN_OF_MODE`, keyed in the TypeScript's order.
fn deps_origin_table() -> IndexMap<&'static str, DepsOrigin> {
    [
        ("copy", InstallMode::Copy),
        ("in-place", InstallMode::InPlace),
        ("link-in-cache", InstallMode::LinkInCache),
    ]
    .into_iter()
    .map(|(name, mode)| (name, deps_origin_of_mode(mode)))
    .collect()
}

/// A lockfile Claude Code recognises and skips the install for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnsupportedLockfile {
    /// `bun.lockb`.
    #[serde(rename = "bun.lockb")]
    BunLockb,
    /// `yarn.lock`.
    #[serde(rename = "yarn.lock")]
    YarnLock,
    /// `pnpm-lock.yaml`.
    #[serde(rename = "pnpm-lock.yaml")]
    PnpmLock,
}

/// The lockfiles Claude Code recognises and skips the install for.
pub const UNSUPPORTED_LOCKFILES: [UnsupportedLockfile; 3] = [
    UnsupportedLockfile::BunLockb,
    UnsupportedLockfile::YarnLock,
    UnsupportedLockfile::PnpmLock,
];

/// "Claude Code stops an install that runs longer and treats it as failed."
pub const INSTALL_TIMEOUT_MS: f64 = 60_000.0;

/// Why the install is skipped, with the names each reason is about.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "reason",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum SkipWhy {
    /// A plugin loaded in place: its dependencies resolve against this source directory.
    LoadedInPlace {
        /// The source directory.
        resolves_from: String,
    },
    /// Link mode.
    LinkMode,
    /// No root `package.json`.
    NoPackageJson,
    /// A `package.json` that is not a JSON object.
    PackageJsonUnreadable,
    /// No supported lockfile.
    NoLockfile,
    /// Lockfiles Claude Code does not install from.
    UnsupportedLockfile {
        /// Those found.
        lockfiles: Vec<UnsupportedLockfile>,
    },
    /// A lockfile format it does not read; `version` is absent when there is no readable
    /// `lockfileVersion`.
    LockfileFormat {
        /// The lockfile.
        lockfile: LockfileName,
        /// Its `lockfileVersion`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<f64>,
    },
    /// The package manager is not on PATH.
    PackageManagerMissing {
        /// Which.
        manager: PackageManager,
    },
    /// `overrides`.
    Overrides,
    /// `patchedDependencies`.
    PatchedDependencies,
    /// A dependency not from the registry, each as `name: reason`.
    NonRegistryDependency {
        /// Those.
        dependencies: Vec<String>,
    },
    /// Each as `name@version`, or the name alone when the lockfile gives no version.
    UnpinnedDependency {
        /// Those.
        packages: Vec<String>,
    },
    /// Downloads not over `https`.
    InsecureDownload {
        /// Their links.
        links: Vec<String>,
    },
    /// Each as `section: names`.
    LockfileMismatch {
        /// Those.
        differences: Vec<String>,
    },
    /// The install folder is not usable.
    InstallFolder {
        /// The folder.
        install_dir: String,
    },
}

/// The reasons that carry nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BareReason {
    /// Link mode.
    LinkMode,
    /// No root `package.json`.
    NoPackageJson,
    /// A `package.json` that is not a JSON object.
    PackageJsonUnreadable,
    /// No supported lockfile.
    NoLockfile,
    /// `overrides`.
    Overrides,
    /// `patchedDependencies`.
    PatchedDependencies,
}

/// `skip`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Skip {
    /// The only value.
    Skip,
}

/// A skip and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeDepsSkip {
    /// `skip`.
    pub action: Skip,
    /// The reason and its fields.
    #[serde(flatten)]
    pub why: SkipWhy,
}

impl From<SkipWhy> for NodeDepsSkip {
    fn from(why: SkipWhy) -> Self {
        Self {
            action: Skip::Skip,
            why,
        }
    }
}

/// A file the install folder takes, or `node_modules` moved back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedFile {
    /// Where it is.
    pub from: String,
    /// Where it goes.
    pub to: String,
}

/// `install`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Install {
    /// The only value.
    Install,
}

/// `copied-version-directory`, the one reason an install runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallReason {
    /// The only value.
    CopiedVersionDirectory,
}

/// The run of the package manager.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRun {
    /// The `exec` form of the command.
    pub spec: CommandSpec,
    /// The separate install folder.
    pub cwd: String,
    /// Its limit.
    pub timeout_ms: f64,
}

/// On success, `node_modules` moves from the install folder into the plugin and replaces one
/// already there (judgment).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveBack {
    /// Where it is.
    pub from: String,
    /// Where it goes.
    pub to: String,
    /// Always `true`.
    pub replace_existing: bool,
}

/// The install folder and what crosses it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStage {
    /// The folder.
    pub directory: String,
    /// The only files the install folder holds: the dependency list and its lockfile.
    pub copy: Vec<StagedFile>,
    /// `node_modules` back into the plugin.
    pub move_back: MoveBack,
}

/// An install to run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDepsInstall {
    /// `install`.
    pub action: Install,
    /// Why it runs.
    pub reason: InstallReason,
    /// Which runs it.
    pub package_manager: PackageManager,
    /// The lockfile it installs from.
    pub lockfile: LockfileName,
    /// The versions the lockfile pins, exactly: always `true`.
    pub frozen: bool,
    /// No `preinstall`, `install` or `postinstall` script runs: always `true`.
    pub ignore_scripts: bool,
    /// The command.
    pub run: InstallRun,
    /// The install folder.
    pub stage: InstallStage,
}

/// An install or a skip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodeDepsPlan {
    /// An install.
    Install(Box<NodeDepsInstall>),
    /// A skip.
    Skip(NodeDepsSkip),
}

/// The plan and what is wrong.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeDepsResult {
    /// The plan.
    pub plan: NodeDepsPlan,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// A skip that carries nothing but its reason.
#[must_use]
pub const fn skip_for(reason: BareReason) -> NodeDepsSkip {
    let why = match reason {
        BareReason::LinkMode => SkipWhy::LinkMode,
        BareReason::NoPackageJson => SkipWhy::NoPackageJson,
        BareReason::PackageJsonUnreadable => SkipWhy::PackageJsonUnreadable,
        BareReason::NoLockfile => SkipWhy::NoLockfile,
        BareReason::Overrides => SkipWhy::Overrides,
        BareReason::PatchedDependencies => SkipWhy::PatchedDependencies,
    };
    NodeDepsSkip {
        action: Skip::Skip,
        why,
    }
}

/// Whether the plan is a skip.
#[must_use]
pub const fn is_skip(plan: &NodeDepsPlan) -> bool {
    matches!(plan, NodeDepsPlan::Skip(_))
}

/// A plan as `isSkip` reads it: its `action` alone.
#[derive(Deserialize)]
struct ActionOf {
    action: String,
}

crate::registry::exports! {
    "marketplace/node-deps-types" {
        "DEPS_ORIGIN_OF_MODE" => ok deps_origin_table;
        "UNSUPPORTED_LOCKFILES" => ok || UNSUPPORTED_LOCKFILES;
        "INSTALL_TIMEOUT_MS" => ok || INSTALL_TIMEOUT_MS;
        "skipFor" => ok |reason: BareReason| skip_for(reason);
        "isSkip" => ok |plan: ActionOf| plan.action == "skip";
    }
}
