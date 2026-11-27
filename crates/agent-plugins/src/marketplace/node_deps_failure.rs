//! Port of `src/marketplace/node-deps-failure.ts`, switched.
//!
//! What the user sees when the dependency install fails or cannot run, and that the plugin still
//! loads (plugins/loading.md, "When the dependency install fails or is skipped": "the plugin still
//! loads, but the parts of it that need the missing packages may not work"; plugins/
//! troubleshooting.md, "The packages it lists are not installed" / "were not installed, because
//! ..."). The docs give the two openings of the note, the command it gives and the example
//! reasons; the words that join them are the kit's (spec/judgments.md).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::node_deps_types::{
    INSTALL_TIMEOUT_MS, NodeDepsPlan, NodeDepsSkip, ReadPackageJson, SkipWhy, UnsupportedLockfile,
};
use crate::format::issues::{PluginIssue, warning};
use crate::js::{number_to_string, trim};
use crate::ports::command_runner::CommandResult;

// ---- the reasons an install cannot run ----

/// What to do about each unsupported lockfile: "Ship a text `bun.lock` or an npm lockfile
/// instead" / "replace it with an npm lockfile".
const fn lockfile_advice(name: UnsupportedLockfile) -> &'static str {
    match name {
        UnsupportedLockfile::BunLockb => {
            "it has a bun.lockb lockfile, Bun's binary format, which Claude Code cannot check; the plugin's author has to ship a text bun.lock or an npm lockfile instead"
        }
        UnsupportedLockfile::YarnLock => {
            "it has a yarn.lock lockfile, which Claude Code does not install from; the plugin’s author has to replace it with an npm lockfile"
        }
        UnsupportedLockfile::PnpmLock => {
            "it has a pnpm-lock.yaml lockfile, which Claude Code does not install from; the plugin’s author has to replace it with an npm lockfile"
        }
    }
}

fn listed(names: &[String]) -> String {
    names.join("; ")
}

/// The clause after "because", per reason, over the fields that reason carries.
fn clauses(why: &SkipWhy) -> String {
    match why {
        SkipWhy::LoadedInPlace { .. } => {
            "it loads in place and Claude Code does not install packages into a source directory"
                .to_owned()
        }
        SkipWhy::LinkMode => "a link-mode plugin gets no install; the directory it links to has to contain the packages in node_modules already".to_owned(),
        SkipWhy::NoPackageJson => "it has no package.json".to_owned(),
        SkipWhy::PackageJsonUnreadable => "its package.json is not a readable JSON object".to_owned(),
        SkipWhy::NoLockfile => {
            "it has no bun.lock, npm-shrinkwrap.json or package-lock.json lockfile".to_owned()
        }
        SkipWhy::UnsupportedLockfile { lockfiles } => lockfiles
            .iter()
            .map(|name| lockfile_advice(*name))
            .collect::<Vec<_>>()
            .join("; and "),
        SkipWhy::LockfileFormat { lockfile, version } => format!(
            "its {} has a lockfileVersion of {} that Claude Code does not read (npm needs 2 or 3, bun.lock no higher than 2)",
            lockfile.file_name(),
            version.map_or_else(|| "none".to_owned(), number_to_string)
        ),
        SkipWhy::PackageManagerMissing { manager } => format!(
            "its lockfile needs {0}, and {0} is not installed on this computer; install it, then update the plugin",
            manager.as_str()
        ),
        SkipWhy::Overrides => {
            "its package.json sets npm overrides, which the install does not apply".to_owned()
        }
        SkipWhy::PatchedDependencies => {
            "its package.json sets Bun patchedDependencies, which the install does not apply"
                .to_owned()
        }
        SkipWhy::NonRegistryDependency { dependencies } => format!(
            "it has dependencies that are not registry packages ({}); only registry packages pinned to an exact version are installed",
            listed(dependencies)
        ),
        SkipWhy::UnpinnedDependency { packages } => format!(
            "it has packages that are not pinned to an exact version in its lockfile ({}); only registry packages pinned to an exact version are installed",
            listed(packages)
        ),
        SkipWhy::InsecureDownload { links } => format!(
            "its lockfile has a download link that is not https ({})",
            listed(links)
        ),
        SkipWhy::LockfileMismatch { differences } => format!(
            "its package.json and its lockfile do not list the same dependencies ({})",
            listed(differences)
        ),
        SkipWhy::InstallFolder { install_dir } => {
            format!("the install folder {install_dir} lies inside the plugin")
        }
    }
}

/// Whether the plugin has a cached copy that could show the note (a plugin loaded in place has
/// none).
const fn cached(why: &SkipWhy) -> bool {
    !matches!(why, SkipWhy::LoadedInPlace { .. })
}

/// Why an install cannot run, as the clause that follows "because" in the note (`it has no
/// package.json`): for a host that words its own message, such as `Node.js packages were not
/// installed: <clause>`.
#[must_use]
pub fn skip_reason_text(skip: &NodeDepsSkip) -> String {
    clauses(&skip.why)
}

// ---- the note ----

/// Which note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoteKind {
    /// `are not installed`.
    NotInstalled,
    /// `were not installed, because ...`.
    NotInstalledBecause,
}

/// The note on a plugin whose packages are missing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDepsNote {
    /// Which note.
    pub kind: NoteKind,
    /// The note `/plugin` and `claude plugin list` show.
    pub text: String,
    /// "run the `claude plugin update` command that the note gives"; absent when updating cannot
    /// help.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_command: Option<String>,
    /// What to do when the automatic install cannot provide the packages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advice: Option<String>,
    /// The note as a finding: a warning, because the plugin still loads.
    pub issues: Vec<PluginIssue>,
}

/// The cached copy of an enabled plugin, as far as the note goes.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedCopy {
    /// Whether the plugin is enabled.
    pub enabled: bool,
    /// Any lockfile is at the plugin root, supported or not.
    pub has_lockfile: bool,
    /// Its `package.json`.
    #[serde(default)]
    pub package_json: Option<ReadPackageJson>,
    /// Whether `node_modules` is there.
    pub has_node_modules: bool,
}

fn non_empty(map: Option<&IndexMap<String, String>>) -> bool {
    map.is_some_and(|map| !map.is_empty())
}

/// "a `package.json` that lists runtime dependencies": `dependencies` and `optionalDependencies`,
/// not dev ones (judgment).
fn lists_runtime_dependencies(package_json: Option<&ReadPackageJson>) -> bool {
    match package_json {
        Some(ReadPackageJson::Json(package_json)) => {
            non_empty(package_json.dependencies.as_ref())
                || non_empty(package_json.optional_dependencies.as_ref())
        }
        Some(ReadPackageJson::Unreadable(_)) | None => false,
    }
}

const FALLBACK: &str = "Install them from a hook into ${CLAUDE_PLUGIN_DATA}: packages that need their lifecycle scripts to build, Python dependencies, plugins locked with Yarn or pnpm and dependencies that are not registry packages are all out of reach of the automatic install.";

fn note(
    id: &str,
    kind: NoteKind,
    text: String,
    update_command: Option<String>,
    advice: Option<String>,
) -> NodeDepsNote {
    let issues = vec![warning(id, &text)];
    NodeDepsNote {
        kind,
        text,
        update_command,
        advice,
        issues,
    }
}

fn unfinished(id: &str) -> NodeDepsNote {
    let update_command = format!("claude plugin update {id}");
    note(
        id,
        NoteKind::NotInstalled,
        format!("The packages it lists are not installed. To retry, run: {update_command}"),
        Some(update_command),
        None,
    )
}

fn cannot_run(id: &str, skip: &NodeDepsSkip) -> NodeDepsNote {
    note(
        id,
        NoteKind::NotInstalledBecause,
        format!(
            "The packages it lists were not installed, because {}.",
            skip_reason_text(skip)
        ),
        None,
        Some(FALLBACK.to_owned()),
    )
}

/// The note on an enabled plugin whose cached copy has a lockfile and a `package.json` that lists
/// runtime dependencies, but no `node_modules`: `are not installed` when the install can run and
/// did not finish, `were not installed, because ...` when it cannot run. `plan` is what
/// `planNodeDepsInstall` says for the copy now; none for a plugin that has no cached copy.
#[must_use]
pub fn node_deps_note(copy: &CachedCopy, plan: &NodeDepsPlan, id: &str) -> Option<NodeDepsNote> {
    let shown = copy.enabled
        && copy.has_lockfile
        && lists_runtime_dependencies(copy.package_json.as_ref())
        && !copy.has_node_modules;
    if !shown {
        return None;
    }
    match plan {
        NodeDepsPlan::Install(_) => Some(unfinished(id)),
        NodeDepsPlan::Skip(skip) => cached(&skip.why).then(|| cannot_run(id, skip)),
    }
}

/// Whether `claude plugin update` retries the install for this note.
const fn retries(kind: NoteKind) -> bool {
    match kind {
        // "To retry the install, run the `claude plugin update` command that the note gives"
        NoteKind::NotInstalled => true,
        // "Updating the plugin doesn't install the packages while that reason stands."
        NoteKind::NotInstalledBecause => false,
    }
}

/// "While `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is set, `claude plugin update` and `/plugin`
/// skip the retry and report that the plugin is at its latest version."
#[must_use]
pub const fn update_retries_install(kind: NoteKind, nonessential_traffic_disabled: bool) -> bool {
    retries(kind) && !nonessential_traffic_disabled
}

// ---- a run that failed ----

/// Why a run failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeDepsFailureCause {
    /// The package manager did not start.
    DidNotStart,
    /// It ran too long.
    TimedOut,
    /// It was aborted.
    Aborted,
    /// It exited with a code other than 0.
    ExitCode,
}

/// What a finished run means.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum InstallOutcome {
    /// The packages are installed.
    Installed,
    /// It failed.
    Failed {
        /// Why.
        cause: NodeDepsFailureCause,
        /// What the user is told, with the end of the output.
        message: String,
    },
}

const OUTPUT_LINES: usize = 20;

fn output_tail(result: &CommandResult) -> String {
    let all = format!("{}{}", result.stdout, result.stderr);
    let lines: Vec<&str> = trim(&all).split('\n').collect();
    let start = lines.len().saturating_sub(OUTPUT_LINES);
    lines.get(start..).unwrap_or_default().join("\n")
}

/// The cause of a run, in the order they are told apart (`CAUSES`), and its description.
fn cause_of(result: &CommandResult) -> Option<(NodeDepsFailureCause, String)> {
    if result.start_error.is_some() {
        let start_error = result.start_error.as_deref().unwrap_or_default();
        return Some((
            NodeDepsFailureCause::DidNotStart,
            format!("The package manager did not start: {start_error}"),
        ));
    }
    if result.timed_out {
        // "Claude Code stops an install that runs longer and treats it as failed."
        return Some((
            NodeDepsFailureCause::TimedOut,
            format!(
                "The dependency install ran longer than {} seconds and was stopped",
                number_to_string(INSTALL_TIMEOUT_MS / 1000.0)
            ),
        ));
    }
    match result.exit_code {
        None => Some((
            NodeDepsFailureCause::Aborted,
            "The dependency install was aborted".to_owned(),
        )),
        Some(0) => None,
        Some(code) => Some((
            NodeDepsFailureCause::ExitCode,
            format!("The dependency install failed with exit code {code}"),
        )),
    }
}

/// What a finished run means: installed, or failed with its cause and the end of its output.
#[must_use]
pub fn install_outcome(result: &CommandResult) -> InstallOutcome {
    let Some((cause, described)) = cause_of(result) else {
        return InstallOutcome::Installed;
    };
    let tail = output_tail(result);
    let message = if tail.is_empty() {
        format!("{described}.")
    } else {
        format!("{described}.\n{tail}")
    };
    InstallOutcome::Failed { cause, message }
}

/// A note as `updateRetriesInstall` reads it: its `kind` alone.
#[derive(Deserialize)]
struct KindOf {
    kind: NoteKind,
}

crate::registry::exports! {
    "marketplace/node-deps-failure" {
        "skipReasonText" => ok |skip: NodeDepsSkip| skip_reason_text(&skip);
        "nodeDepsNote" => ok |copy: CachedCopy, plan: NodeDepsPlan, id: String| node_deps_note(&copy, &plan, &id);
        "updateRetriesInstall" => ok |shown: KindOf, nonessential_traffic_disabled: bool|
            update_retries_install(shown.kind, nonessential_traffic_disabled);
        "installOutcome" => ok |result: CommandResult| install_outcome(&result);
    }
}
