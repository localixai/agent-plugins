//! Port of `src/rules/decide/block-exceptions.ts`, switched.
//!
//! The read and write blocks (settings-reference.md,
//! `permissions.blockReadsOutsideWorkingDirectories` and
//! `permissions.blockWritesOutsideWorkingDirectories`): what they refuse and the files Claude Code
//! itself needs, which stay open.

use serde::Deserialize;

use super::context::ProtectedPathOptions;
use super::protected_paths::{config_directory_of, is_own_file, is_stored_remote_settings};
use super::scope::{in_scope, posix_of};
use super::write_block_messages::WriteBlockReason;
use crate::registry::Port;
use crate::rules::path::posix::{fold_path, relative_within};
use crate::rules::types::{MatchContext, PathContext};

/// The members of `DecideContext` the blocks read.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockContext {
    /// `match`.
    #[serde(rename = "match")]
    pub match_context: MatchContext,
    /// `protectedPaths`.
    #[serde(default)]
    pub protected_paths: Option<ProtectedPathOptions>,
    /// `repositoryDirectories`.
    #[serde(default)]
    pub repository_directories: Option<Vec<String>>,
    /// `blockReadsOutsideWorkingDirectories`.
    #[serde(default)]
    pub block_reads_outside_working_directories: Option<bool>,
    /// `blockWritesOutsideWorkingDirectories`.
    #[serde(default)]
    pub block_writes_outside_working_directories: Option<bool>,
    /// `managedSettingsFolderUnresolved`.
    #[serde(default)]
    pub managed_settings_folder_unresolved: Option<bool>,
    /// `sessionDirectories`.
    #[serde(default)]
    pub session_directories: Option<Vec<String>>,
}

/// A `DecideContext` as it crosses: data, or, when it holds a function (`requiresPermission`), a
/// port whose members are read.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum BlockContextGiven {
    /// The host's record, holding a function.
    Host(Port),
    /// Plain data.
    Data(Box<BlockContext>),
}

impl BlockContextGiven {
    /// The members the blocks read.
    pub async fn read(self) -> BlockContext {
        match self {
            Self::Data(data) => *data,
            Self::Host(port) => BlockContext {
                match_context: port.member("match").await,
                protected_paths: port.member("protectedPaths").await,
                repository_directories: port.member("repositoryDirectories").await,
                block_reads_outside_working_directories: port
                    .member("blockReadsOutsideWorkingDirectories")
                    .await,
                block_writes_outside_working_directories: port
                    .member("blockWritesOutsideWorkingDirectories")
                    .await,
                managed_settings_folder_unresolved: port
                    .member("managedSettingsFolderUnresolved")
                    .await,
                session_directories: port.member("sessionDirectories").await,
            },
        }
    }
}

const SETTINGS_NAMES: [&str; 3] = [
    "settings.json",
    "settings.local.json",
    "managed-settings.json",
];

impl BlockContext {
    fn options(&self) -> ProtectedPathOptions {
        self.protected_paths.clone().unwrap_or_default()
    }

    fn paths(&self) -> &PathContext {
        &self.match_context.paths
    }
}

/// `settings.json` and `settings.local.json` in a `.claude` directory or the configuration
/// directory, and `managed-settings.json`.
fn is_named_settings_file(path: &str, paths: &PathContext, ctx: &BlockContext) -> bool {
    let folded = fold_path(path, paths.platform);
    let segments: Vec<&str> = folded
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let name = segments.last().copied().unwrap_or("");
    let parent = segments
        .len()
        .checked_sub(2)
        .and_then(|at| segments.get(at))
        .copied();
    if !SETTINGS_NAMES.contains(&name) {
        return false;
    }
    let configured = config_directory_of(paths, &ctx.options());
    parent == Some(".claude")
        || name == "managed-settings.json"
        || relative_within(&format!("{configured}/{name}"), path, Some(paths.platform)).as_deref()
            == Some("")
}

/// "Claude Code settings files": the named ones, the files the session was started with
/// (`--settings`, `--agents`, `--mcp-config`), the stored server-managed settings and the files
/// kept with it, and "any `.json` file, while Claude Code can't work out where the
/// `managed-settings.d/` folder leads" (errors.md, "Writes to Claude Code settings files are
/// blocked"). The write block refuses these "inside the working directories too, because a
/// settings file can lift the block". The list is the kit's reading (spec/judgments.md, J13.28 of
/// the specification).
fn is_settings_file(path: &str, paths: &PathContext, ctx: &BlockContext) -> bool {
    let options = ctx.options();
    let launched = options
        .files
        .as_deref()
        .unwrap_or_default()
        .iter()
        .any(|file| {
            relative_within(&posix_of(file, paths), path, Some(paths.platform)).as_deref()
                == Some("")
        });
    let any_json = ctx.managed_settings_folder_unresolved == Some(true) && path.ends_with(".json");
    is_named_settings_file(path, paths, ctx)
        || launched
        || is_stored_remote_settings(path, paths, &options)
        || any_json
}

/// The directories of the configuration directory that "Files Claude Code itself needs" are read
/// from.
const READ_DIRECTORIES: [&str; 5] = ["skills", "plugins", "rules", "agents", "commands"];

fn in_session_directory(path: &str, ctx: &BlockContext) -> bool {
    let paths = ctx.paths();
    ctx.session_directories
        .as_deref()
        .unwrap_or_default()
        .iter()
        .any(|directory| {
            relative_within(&posix_of(directory, paths), path, Some(paths.platform)).is_some()
        })
}

/// Whether the read block leaves `path` readable although it is outside the working directories:
/// "your skills, plugins, rules, agents, commands, and the `CLAUDE.md` memory file under
/// `~/.claude/`", and the session's own directories.
fn readable_under_block(path: &str, ctx: &BlockContext) -> bool {
    let paths = ctx.paths();
    let configured = config_directory_of(paths, &ctx.options());
    let below = relative_within(&configured, path, Some(paths.platform));
    let own = below
        .as_deref()
        .and_then(|below| below.split('/').next())
        .is_some_and(|top| top == "CLAUDE.md" || READ_DIRECTORIES.contains(&top));
    own || in_session_directory(path, ctx)
}

/// Whether the write block leaves `path` writable although it is outside the working directories:
/// "plan files, the scratchpad, a background session's `tmp` directory, and `.md` files in this
/// project's auto memory".
fn writable_under_block(path: &str, ctx: &BlockContext) -> bool {
    is_own_file(path, ctx.paths(), &ctx.options()) || in_session_directory(path, ctx)
}

/// The working directories as the blocks count them: a directory that only repository settings
/// add "doesn't count" (settings-reference.md, `permissions.blockReadsOutsideWorkingDirectories`).
#[must_use]
pub fn block_paths(ctx: &BlockContext) -> PathContext {
    let paths = ctx.paths();
    let excluded: Vec<String> = ctx
        .repository_directories
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|directory| posix_of(directory, paths))
        .collect();
    let mut counted = paths.clone();
    if !excluded.is_empty() {
        counted
            .additional_directories
            .retain(|directory| !excluded.contains(&posix_of(directory, paths)));
    }
    counted
}

/// Why the write block refuses `paths`, or `None` when it lets them through.
#[must_use]
pub fn write_block_reason(paths: &[String], ctx: &BlockContext) -> Option<WriteBlockReason> {
    if ctx.block_writes_outside_working_directories != Some(true) {
        return None;
    }
    if paths
        .iter()
        .any(|path| is_settings_file(path, ctx.paths(), ctx))
    {
        return Some(WriteBlockReason::SettingsFile);
    }
    let counted = block_paths(ctx);
    let outside = paths
        .iter()
        .any(|path| !in_scope(path, &counted) && !writable_under_block(path, ctx));
    outside.then_some(WriteBlockReason::OutsideWorkingDirectory)
}

/// Whether the read block refuses a read of `paths`.
#[must_use]
pub fn read_blocked(paths: &[String], ctx: &BlockContext) -> bool {
    ctx.block_reads_outside_working_directories == Some(true) && {
        let counted = block_paths(ctx);
        paths
            .iter()
            .any(|path| !in_scope(path, &counted) && !readable_under_block(path, ctx))
    }
}

crate::registry::exports! {
    "rules/decide/block-exceptions" {
        "blockPaths" => async ok |ctx: BlockContextGiven| block_paths(&ctx.read().await);
        "writeBlockReason" => async ok |paths: Vec<String>, ctx: BlockContextGiven| write_block_reason(&paths, &ctx.read().await);
        "readBlocked" => async ok |paths: Vec<String>, ctx: BlockContextGiven| read_blocked(&paths, &ctx.read().await);
    }
}
