//! Port of `src/rules/decide/protected-paths.ts`, switched.
//!
//! permission-modes.md, "Protected paths": "Protected directories" and "Protected files". A
//! directory matches wherever it sits in a path (`.git` of the project and of a nested folder);
//! `.config/git` is two segments; a file matches by its name. The lists are the docs' own.

use super::context::ProtectedPathOptions;
use super::scope::posix_of;
use crate::rules::path::posix::{fold_path, relative_within};
use crate::rules::types::PathContext;

const DIRECTORIES: [&[&str]; 9] = [
    &[".git"],
    &[".config", "git"],
    &[".vscode"],
    &[".idea"],
    &[".husky"],
    &[".cargo"],
    &[".devcontainer"],
    &[".yarn"],
    &[".mvn"],
];

const FILES: [&str; 37] = [
    ".gitconfig",
    ".gitmodules",
    ".bashrc",
    ".bash_profile",
    ".bash_login",
    ".bash_aliases",
    ".bash_logout",
    ".zshrc",
    ".zprofile",
    ".zshenv",
    ".zlogin",
    ".zlogout",
    ".profile",
    ".envrc",
    ".npmrc",
    ".yarnrc",
    ".yarnrc.yml",
    ".pnp.cjs",
    ".pnp.loader.mjs",
    ".pnpmfile.cjs",
    "bunfig.toml",
    ".bunfig.toml",
    ".bazelrc",
    ".bazelversion",
    ".bazeliskrc",
    ".pre-commit-config.yaml",
    "lefthook.yml",
    "lefthook.yaml",
    ".lefthook.yml",
    ".lefthook.yaml",
    "gradle-wrapper.properties",
    "maven-wrapper.properties",
    ".devcontainer.json",
    ".ripgreprc",
    "pyrightconfig.json",
    ".mcp.json",
    ".claude.json",
];

const CLAUDE: &str = ".claude";

/// The names of `path`, as the platform compares them (`foldPath`).
fn segments_of(path: &str, paths: &PathContext) -> Vec<String> {
    fold_path(path, paths.platform)
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether a host path and a kit path are the same file on the platform.
fn same_path(file: &str, path: &str, paths: &PathContext) -> bool {
    relative_within(&posix_of(file, paths), path, Some(paths.platform)).as_deref() == Some("")
}

fn starts_at(segments: &[String], at: usize, names: &[&str]) -> bool {
    names.iter().enumerate().all(|(offset, name)| {
        segments
            .get(at + offset)
            .is_some_and(|segment| segment == name)
    })
}

fn has_directory(segments: &[String]) -> bool {
    DIRECTORIES
        .iter()
        .any(|names| (0..segments.len()).any(|at| starts_at(segments, at, names)))
}

/// The exceptions below `.claude` that follow the session: its plan files and its memory
/// (restricted or not).
struct ClaudeContext<'a> {
    path: &'a str,
    /// The segments after the first `.claude`.
    rest: Vec<String>,
    home_claude: String,
    options: &'a ProtectedPathOptions,
    paths: &'a PathContext,
}

impl ClaudeContext<'_> {
    fn rest_is(&self, at: usize, name: &str) -> bool {
        self.rest.get(at).is_some_and(|segment| segment == name)
    }

    fn below(&self, folder: &str) -> bool {
        relative_within(
            &format!("{}/{folder}", self.home_claude),
            self.path,
            Some(self.paths.platform),
        )
        .is_some()
    }

    fn restricted(&self) -> bool {
        self.options.restricted == Some(true)
    }
}

fn is_markdown(path: &str) -> bool {
    path.ends_with(".md")
}

type Exception = fn(&ClaudeContext<'_>) -> bool;

/// Each exception of "`.claude`, with a few exceptions": Claude's own worktrees, the session's own
/// plan files (in `~/.claude/plans/` or the `plansDirectory`), a background session's scratch
/// directory, and Markdown in the auto memory and subagent memory directories of a session started
/// without `--restricted`.
const OWN_FILES: [Exception; 5] = [
    |claude| claude.below("plans"),
    |claude| {
        claude
            .options
            .plans_directory
            .as_ref()
            .is_some_and(|plans| {
                relative_within(
                    &posix_of(plans, claude.paths),
                    claude.path,
                    Some(claude.paths.platform),
                )
                .is_some()
            })
    },
    |claude| claude.below("jobs") && claude.rest_is(2, "tmp"),
    |claude| {
        !claude.restricted()
            && is_markdown(claude.path)
            && claude.below("projects")
            && claude.rest_is(2, "memory")
    },
    |claude| !claude.restricted() && is_markdown(claude.path) && claude.rest_is(0, "agent-memory"),
];

fn claude_exceptions(claude: &ClaudeContext<'_>) -> bool {
    claude.rest_is(0, "worktrees") || OWN_FILES.iter().any(|exception| exception(claude))
}

/// Where Claude Code keeps its own files: `~/.claude`, or the directory `CLAUDE_CONFIG_DIR` points
/// to.
#[must_use]
pub fn config_directory_of(paths: &PathContext, options: &ProtectedPathOptions) -> String {
    options.config_directory.as_ref().map_or_else(
        || format!("{}/{CLAUDE}", posix_of(&paths.home, paths)),
        |directory| posix_of(directory, paths),
    )
}

/// The segments of `path` below the configuration directory, or below the first `.claude`, or
/// none.
fn rest_of(
    segments: &[String],
    path: &str,
    config_directory: &str,
    paths: &PathContext,
) -> Vec<String> {
    if let Some(below) = relative_within(config_directory, path, Some(paths.platform)) {
        return below
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_owned)
            .collect();
    }
    segments
        .iter()
        .position(|segment| segment == CLAUDE)
        .map_or_else(Vec::new, |at| {
            segments.get(at + 1..).unwrap_or_default().to_vec()
        })
}

fn claude_context<'a>(
    segments: &[String],
    path: &'a str,
    paths: &'a PathContext,
    options: &'a ProtectedPathOptions,
) -> ClaudeContext<'a> {
    let home_claude = config_directory_of(paths, options);
    ClaudeContext {
        path,
        rest: rest_of(segments, path, &home_claude, paths),
        home_claude,
        options,
        paths,
    }
}

/// Whether `path` is one of Claude Code's own files for the session: its plan files, a background
/// session's scratch directory, and Markdown in the auto memory and subagent memory directories
/// (the files the write block leaves writable, settings-reference.md, "What the write block
/// refuses and what stays writable"). The kit cannot tell the session's own plan file from another
/// one (spec/judgments.md).
#[must_use]
pub fn is_own_file(path: &str, paths: &PathContext, options: &ProtectedPathOptions) -> bool {
    let segments = segments_of(path, paths);
    let claude = claude_context(&segments, path, paths, options);
    OWN_FILES.iter().any(|exception| exception(&claude))
}

fn protected_by_claude(
    segments: &[String],
    path: &str,
    paths: &PathContext,
    options: &ProtectedPathOptions,
) -> bool {
    if !segments.iter().any(|segment| segment == CLAUDE) {
        return false;
    }
    let claude = claude_context(segments, path, paths, options);
    !claude_exceptions(&claude)
}

/// "`~/.claude/dev-mods/`, or `dev-mods/` in the directory `CLAUDE_CONFIG_DIR` points to".
fn in_dev_mods(path: &str, paths: &PathContext, options: &ProtectedPathOptions) -> bool {
    relative_within(
        &format!("{}/dev-mods", config_directory_of(paths, options)),
        path,
        Some(paths.platform),
    )
    .is_some()
}

const REMOTE_SETTINGS: &str = "remote-settings.json";

/// The paths of an optional list, none when it is absent.
fn listed(list: Option<&Vec<String>>) -> &[String] {
    list.map_or(&[], Vec::as_slice)
}

/// The stored copy of server-managed settings "and the signature, sync, and consent files Claude
/// Code keeps with it, in `~/.claude` or the directory `CLAUDE_CONFIG_DIR` points to"
/// (permission-modes.md, "Protected paths"): `remote-settings.json` of the configuration
/// directory, and the files the host names (`remoteSettingsFiles`). Judgment (spec/judgments.md,
/// Agent Plugins Extended J13.35): the docs name the kept files by role only, so none is guessed.
#[must_use]
pub fn is_stored_remote_settings(
    path: &str,
    paths: &PathContext,
    options: &ProtectedPathOptions,
) -> bool {
    let below = relative_within(
        &config_directory_of(paths, options),
        path,
        Some(paths.platform),
    );
    let stored = below.is_some_and(|below| fold_path(&below, paths.platform) == REMOTE_SETTINGS);
    stored
        || listed(options.remote_settings_files.as_ref())
            .iter()
            .any(|file| same_path(file, path, paths))
}

/// Whether `path` is the target of a symlinked settings file (`options.symlinkTargets`) and is not
/// protected for another reason: "If the file's own path is also that of a settings file ... the
/// write goes to the classifier as other protected-path writes do".
#[must_use]
pub fn is_symlink_target_only(
    path: &str,
    paths: &PathContext,
    options: &ProtectedPathOptions,
) -> bool {
    let target = listed(options.symlink_targets.as_ref())
        .iter()
        .any(|file| same_path(file, path, paths));
    target && !is_protected_path(path, paths, options)
}

/// Whether a write to `path` (absolute, POSIX) is a write to a protected path: a protected
/// directory or file, `.claude` apart from its exceptions, a file the session was started with
/// (`--settings`, `--mcp-config`, `--agents`), a directory loaded with `--plugin-dir`, or
/// `dev-mods`. The target of a symlinked settings file is `isSymlinkTargetOnly`.
#[must_use]
pub fn is_protected_path(path: &str, paths: &PathContext, options: &ProtectedPathOptions) -> bool {
    let segments = segments_of(path, paths);
    let name = segments.last().map_or("", String::as_str);
    has_directory(&segments)
        || FILES.contains(&name)
        || started_with(path, paths, options)
        || in_dev_mods(path, paths, options)
        || is_stored_remote_settings(path, paths, options)
        || protected_by_claude(&segments, path, paths, options)
}

/// A file the session was started with, or a directory loaded with `--plugin-dir`.
fn started_with(path: &str, paths: &PathContext, options: &ProtectedPathOptions) -> bool {
    if listed(options.files.as_ref())
        .iter()
        .any(|file| same_path(file, path, paths))
    {
        return true;
    }
    listed(options.directories.as_ref())
        .iter()
        .any(|directory| {
            relative_within(&posix_of(directory, paths), path, Some(paths.platform)).is_some()
        })
}

crate::registry::exports! {
    "rules/decide/protected-paths" {
        "isSymlinkTargetOnly" => ok |path: String, paths: PathContext, options: ProtectedPathOptions| is_symlink_target_only(&path, &paths, &options);
        "isProtectedPath" => ok |path: String, paths: PathContext, options: Option<ProtectedPathOptions>| is_protected_path(&path, &paths, &options.unwrap_or_default());
    }
}
