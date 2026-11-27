//! Port of `src/settings/permissions/sandbox-read-block.ts`, switched.
//!
//! What `permissions.blockReadsOutsideWorkingDirectories` does to sandboxed commands (settings-reference.md,
//! "Sandboxed commands under the block"). Pure over the facts the host reads; the sandbox is the host's.

use serde::{Deserialize, Serialize};

use crate::format::path_text::is_within;
use crate::registry::Set;

/// "Your home directory and ... the other roots that hold user files."
pub const BLOCKED_USER_ROOTS: [&str; 8] = [
    "/Users",
    "/home",
    "/root",
    "/Volumes",
    "/mnt",
    "/media",
    "/run/media",
    "/srv",
];

/// What the host reads of the session and its sandbox.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxBlockInput {
    /// The block key is `true` in some file.
    pub block_reads: bool,
    /// `sandbox.filesystem.disabled`.
    pub filesystem_disabled: bool,
    /// `allowManagedReadPathsOnly` is set.
    pub managed_read_paths_only: bool,
    /// "The path of the directory you started Claude Code in contains a glob character".
    pub start_directory: String,
    /// The home directory of the user.
    pub home: String,
    /// What the block re-opens: "the working directories, worktrees Claude Code creates in the
    /// session, the session temp directory, and the parts of `~/.claude` that commands need". The host
    /// knows each; the kit returns them as `reopened`.
    #[serde(default)]
    pub reopened: Option<Vec<String>>,
    /// The repository's common `.git` directory when the session's working directory is a linked git
    /// worktree, "including one Claude Code entered mid-session"; absent otherwise.
    #[serde(default)]
    pub worktree_common_git_dir: Option<String>,
}

/// What the block does to sandboxed commands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxBlock {
    /// Whether the block reaches sandboxed commands; Claude's file tools enforce it either way.
    pub reaches_sandbox: bool,
    /// Roots a sandboxed command may not read.
    pub deny_read: Vec<String>,
    /// What a sandboxed command may read again below the denied roots.
    pub reopened: Vec<String>,
    /// "The repository's common `.git` directory stays readable and writable to sandboxed commands."
    pub read_write: Vec<String>,
    /// What stays write-denied inside `readWrite`: "Writes to `hooks/` and `config` inside that
    /// directory remain denied" (sandboxing.md, "Filesystem isolation").
    pub deny_write: Vec<String>,
    /// "`allowRead` and `allowWrite` entries from repository settings don't count" while the block is in force.
    pub repository_allow_entries_count: bool,
}

fn off() -> SandboxBlock {
    SandboxBlock {
        reaches_sandbox: false,
        deny_read: Vec::new(),
        reopened: Vec::new(),
        read_write: Vec::new(),
        deny_write: Vec::new(),
        repository_allow_entries_count: true,
    }
}

/// The block's effect on sandboxed commands: it reaches them unless one of the three cases the docs list holds.
#[must_use]
pub fn sandbox_read_block(input: &SandboxBlockInput) -> SandboxBlock {
    let exempt = !input.block_reads
        || input.filesystem_disabled
        || input.managed_read_paths_only
        || input.start_directory.contains(['*', '?', '[']);
    if exempt {
        return off();
    }
    let common_git: Vec<String> = input.worktree_common_git_dir.iter().cloned().collect();
    let mut deny_read = vec![input.home.clone()];
    deny_read.extend(BLOCKED_USER_ROOTS.iter().map(|root| (*root).to_owned()));
    let mut reopened = input.reopened.clone().unwrap_or_default();
    reopened.extend(common_git.iter().cloned());
    SandboxBlock {
        reaches_sandbox: true,
        deny_read,
        reopened,
        read_write: common_git.clone(),
        deny_write: common_git
            .iter()
            .flat_map(|git| [format!("{git}/hooks"), format!("{git}/config")])
            .collect(),
        repository_allow_entries_count: false,
    }
}

/// What a sandboxed command may re-read of the user's git configuration.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitConfigInput {
    /// The home directory.
    pub home: String,
    /// `XDG_CONFIG_HOME`, when set.
    #[serde(default)]
    pub xdg_config_home: Option<String>,
    /// The files the configuration includes.
    pub included: Vec<String>,
    /// Files a sandboxed command can write, directly or through a symlink: not re-opened.
    pub writable: Set<String>,
    /// `sandbox.filesystem.denyRead` entries: they "always take precedence over this re-open".
    pub deny_read: Vec<String>,
}

/// "Under the block, Claude Code re-opens your global git configuration files": the paths it names.
#[must_use]
pub fn git_config_reopened(input: &GitConfigInput) -> Vec<String> {
    let home = &input.home;
    let xdg = input
        .xdg_config_home
        .clone()
        .unwrap_or_else(|| format!("{home}/.config"));
    let mut candidates = vec![
        format!("{home}/.gitconfig"),
        format!("{xdg}/git/config"),
        format!("{xdg}/git/ignore"),
        format!("{xdg}/git/attributes"),
    ];
    candidates.extend(input.included.iter().cloned());
    candidates
        .into_iter()
        .filter(|file| {
            !input.writable.members.contains(file)
                && !input.deny_read.iter().any(|entry| is_within(entry, file))
        })
        .collect()
}

crate::registry::exports! {
    "settings/permissions/sandbox-read-block" {
        "BLOCKED_USER_ROOTS" => ok || BLOCKED_USER_ROOTS;
        "sandboxReadBlock" => ok |input: SandboxBlockInput| sandbox_read_block(&input);
        "gitConfigReopened" => ok |input: GitConfigInput| git_config_reopened(&input);
    }
}
