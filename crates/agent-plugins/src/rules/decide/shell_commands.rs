//! Ported from `src/rules/decide/shell-commands.ts`; only the core uses it now, so it has no wrapper.

use std::collections::HashSet;

use super::context::ProtectedPathOptions;
use super::file_targets::file_operands;
use super::fs_operands::{filesystem_operands, is_filesystem_program};
use super::protected_paths::{is_protected_path, is_symlink_target_only};
use super::read_only_set::{
    GIT_READ_ONLY, READ_ONLY_PROGRAMS, arguments_are_read_only, has_network_argument,
    touches_special_variable,
};
use super::scope::{in_scope, posix_of};
use super::shell_words::{Walk, has_unquoted_glob, operand_path, read_path};
use crate::rules::bash::exec_wrappers::prompts_despite_prefix_rule;
use crate::rules::bash::split::{CommandScan, scan_command};
use crate::rules::bash::words::{ShellCommand, ShellWord};
use crate::rules::bash::wrappers::{read_only_words, stripped_words};
use crate::rules::path::posix::{HostPaths, absolute_path};
use crate::rules::types::{PathContext, PermissionBehavior};

/// What a Bash command is, as far as the modes need it: read-only (permissions.md, "Read-only
/// commands"), or made of the filesystem commands `acceptEdits` approves (permission-modes.md,
/// "Auto-approve file edits with acceptEdits mode"), or neither. Every subcommand is looked at, in
/// the order the command runs them, with a `cd` moving the directory later operands are read
/// against.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's shape: one fact per flag"
)]
pub struct ShellVerdict {
    /// Every subcommand is in the read-only set.
    pub read_only: bool,
    /// Every subcommand is read-only or a filesystem command on paths in scope and not protected.
    pub edits_in_scope: bool,
    /// A filesystem command's operand is a protected path.
    pub writes_protected: bool,
    /// Protected only where a symlink leads, or as the target of a symlinked settings file.
    pub writes_protected_via_link: bool,
    /// A file-reading command's operand is outside the working directories, or cannot be told.
    pub reads_outside: bool,
    /// The parser cannot trace the command: it does not parse, changes directory twice or runs a
    /// subshell.
    pub untraceable: bool,
    /// The subcommands in reading order.
    pub subcommands: Vec<String>,
    /// For each subcommand, whether it is read-only on its own.
    pub read_only_parts: Vec<bool>,
}

/// A command with nothing to read, or one that is not Bash: nothing about it is approved
/// (`UNREAD`, the default).
#[must_use]
pub fn unread() -> ShellVerdict {
    ShellVerdict::default()
}

/// "Commands longer than 10,000 characters always prompt because they exceed what the analysis
/// parses."
const MAX_COMMAND_LENGTH: usize = 10_000;

/// Commands with write-capable or exec-capable flags: "prompt when an unquoted glob is present".
const GLOB_SENSITIVE: [&str; 2] = ["find", "git"];

/// `cd`: read-only when it moves into the working directory or an additional directory. `cd -`
/// goes to a directory the kit cannot know, so it is not read-only (spec/judgments.md).
fn read_only_cd(words: &[ShellWord], walk: &mut Walk<'_>) -> bool {
    let home = absolute_path("~", &HostPaths::from(walk.paths));
    let target = match words.get(1) {
        None => Some(home),
        Some(operand) if operand.value == "-" => None,
        Some(operand) => operand_path(operand, walk),
    };
    let Some(target) = target else { return false };
    if words.len() > 2 || !in_scope(&target, walk.paths) {
        return false;
    }
    walk.changed_directory = walk.changed_directory || target != walk.base;
    walk.base = target;
    true
}

/// A read-only form of `git` (`GIT_READ_ONLY`), with no option before the subcommand.
fn read_only_git(words: &[ShellWord], walk: &mut Walk<'_>) -> bool {
    walk.used_git = true;
    let subcommand = words.get(1).map_or("", |word| word.value.as_str());
    GIT_READ_ONLY
        .iter()
        .find(|(name, _)| *name == subcommand)
        .is_some_and(|(_, form)| form(&values_of(words.get(2..).unwrap_or_default())))
}

fn values_of(words: &[ShellWord]) -> Vec<String> {
    words.iter().map(|word| word.value.clone()).collect()
}

type Special = fn(&[ShellWord], &mut Walk<'_>) -> bool;

const SPECIAL: [(&str, Special); 2] = [("cd", read_only_cd), ("git", read_only_git)];

/// What keeps a known program read-only: no glob that could be a flag, and the docs' prompt cases
/// (redirect targets: `redirects.rs`).
fn arguments_permit(command: &ShellCommand, walk: &Walk<'_>) -> bool {
    let values = values_of(&command.words);
    let program = values.first().map_or("", String::as_str);
    let glob_flag = GLOB_SENSITIVE.contains(&program)
        && has_unquoted_glob(command.words.get(1..).unwrap_or_default());
    !prompts_despite_prefix_rule(command)
        && !glob_flag
        && !has_network_argument(&command.words, walk.paths)
        && arguments_are_read_only(program, &values)
}

/// In the read-only set once the wrappers it sees through are removed (`readOnlyWords`); `cd` and
/// `git` only as written (oracle 2.1.322: `LANG=C git status` and `timeout 5 cd sub` were asked).
fn is_read_only(command: &ShellCommand, walk: &mut Walk<'_>) -> bool {
    let seen = read_only_words(&command.words);
    let program = seen.words.first().map_or("", |word| word.value.as_str());
    let special = SPECIAL
        .iter()
        .find(|(name, _)| *name == program)
        .map(|(_, special)| *special);
    let known = special.map_or_else(|| READ_ONLY_PROGRAMS.contains(&program), |_| !seen.stripped);
    let inner = ShellCommand {
        text: command.text.clone(),
        words: seen.words.clone(),
        redirects: command.redirects.clone(),
    };
    if !known || !arguments_permit(&inner, walk) {
        return false;
    }
    special.is_none_or(|special| special(&seen.words, walk))
}

/// Whether a file-reading command reads outside the working directories, by an operand it names,
/// a glob's directory or a path the symlink check resolved. An operand a variable or substitution
/// hides counts as outside (spec/judgments.md).
fn reads_outside_scope(words: &[ShellWord], walk: &Walk<'_>) -> bool {
    let Some(named) = file_operands(words) else {
        return false;
    };
    // A command that only writes the files it names reads none of them.
    if named.reads.is_empty() && !named.writes.is_empty() {
        return false;
    }
    let reads = named
        .reads
        .iter()
        .map(|operand| read_path(operand, walk))
        .chain(walk.resolved.iter().cloned().map(Some));
    reads
        .into_iter()
        .any(|path| path.is_none_or(|path| !in_scope(&path, walk.paths)))
}

/// A filesystem command and where it writes: `touch` as written, the others past safe prefixes.
fn filesystem_targets(command: &ShellCommand, walk: &Walk<'_>) -> Option<Vec<Option<String>>> {
    let stripped = stripped_words(command, PermissionBehavior::Allow, &HashSet::new());
    let program = stripped.first().map_or("", |word| word.value.as_str());
    let prefixed = stripped.len() != command.words.len();
    if !is_filesystem_program(program) || (program == "touch" && prefixed) {
        return None;
    }
    let operands = filesystem_operands(program, stripped.get(1..).unwrap_or_default())?;
    if operands.is_empty() {
        return None;
    }
    Some(
        operands
            .iter()
            .map(|operand| operand_path(operand, walk))
            .collect(),
    )
}

/// One subcommand's judgment.
#[derive(Clone, Copy, Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's shape: one fact per flag"
)]
pub struct SubVerdict {
    /// `readOnly`.
    pub read_only: bool,
    /// `edits`.
    pub edits: bool,
    /// `protectedWrite`.
    pub protected_write: bool,
    /// `viaLink`.
    pub via_link: bool,
    /// `readsOutside`.
    pub reads_outside: bool,
}

/// A filesystem command's judgment: where it writes is the paths it names and the paths the
/// host's symlink check resolved (permission-modes.md, "Each path also goes through the symlink
/// check").
#[must_use]
pub fn judge_write(targets: &[Option<String>], walk: &Walk<'_>) -> SubVerdict {
    let named: Vec<&str> = targets.iter().flatten().map(String::as_str).collect();
    let protect = |path: &str| is_protected_path(path, walk.paths, walk.options);
    let asked = named.iter().any(|path| protect(path));
    let via_link = !asked
        && (named
            .iter()
            .any(|path| is_symlink_target_only(path, walk.paths, walk.options))
            || walk.resolved.iter().any(|path| protect(path)));
    let inside = named.len() == targets.len()
        && named
            .iter()
            .copied()
            .chain(walk.resolved.iter().map(String::as_str))
            .all(|path| in_scope(path, walk.paths));
    SubVerdict {
        edits: inside && !asked && !via_link,
        protected_write: asked || via_link,
        via_link,
        ..SubVerdict::default()
    }
}

fn judge(command: &ShellCommand, walk: &mut Walk<'_>) -> SubVerdict {
    let denied = stripped_words(command, PermissionBehavior::Deny, &HashSet::new());
    let reads_outside = reads_outside_scope(&denied, walk);
    if is_read_only(command, walk) {
        return SubVerdict {
            read_only: true,
            edits: true,
            reads_outside,
            ..SubVerdict::default()
        };
    }
    let judged = filesystem_targets(command, walk)
        .map_or_else(SubVerdict::default, |targets| judge_write(&targets, walk));
    SubVerdict {
        reads_outside,
        ..judged
    }
}

/// "A command the shell parser can't trace, such as one that changes directory more than once or
/// runs a subshell", and a `cd` with `git` (the chain that the permission checker also cannot
/// trace). A line that defines a function is one too: a later word may run the function's body
/// under the name of a read-only command (spec/judgments.md).
fn is_untraceable(scan: &CommandScan, walk: &Walk<'_>) -> bool {
    let changes = scan
        .commands
        .iter()
        .filter(|command| command.words.first().is_some_and(|word| word.value == "cd"))
        .count();
    changes > 1
        || scan.nests
        || !scan.functions.is_empty()
        || (walk.changed_directory && walk.used_git)
}

/// What the subcommands together are: each flag holds when it holds for every subcommand, or for
/// any one, as it reads. `untraceable` and the parts are left to the caller.
fn combine(verdicts: &[SubVerdict], walk: &Walk<'_>, command: &str, defines: bool) -> ShellVerdict {
    // "`cd` with `git`: prompts when the `cd` changes into a different directory." A function
    // defined on the line may run anything under a read-only name.
    let sound = !(walk.changed_directory && walk.used_git)
        && !touches_special_variable(command)
        && !defines;
    let every_write_via_link = !verdicts
        .iter()
        .any(|verdict| verdict.protected_write && !verdict.via_link);
    ShellVerdict {
        read_only: sound && verdicts.iter().all(|verdict| verdict.read_only),
        edits_in_scope: sound && verdicts.iter().all(|verdict| verdict.edits),
        writes_protected: verdicts.iter().any(|verdict| verdict.protected_write),
        writes_protected_via_link: verdicts.iter().any(|verdict| verdict.via_link)
            && every_write_via_link,
        reads_outside: verdicts.iter().any(|verdict| verdict.reads_outside),
        ..ShellVerdict::default()
    }
}

/// The verdict on a Bash command: see `ShellVerdict`. `resolved` is where the host's symlink
/// check says the call's paths lead.
#[must_use]
pub fn shell_verdict(
    command: &str,
    paths: &PathContext,
    options: &ProtectedPathOptions,
    resolved: &[String],
) -> ShellVerdict {
    let scan = scan_command(command, None);
    if !scan.parseable || command.encode_utf16().count() > MAX_COMMAND_LENGTH {
        return ShellVerdict {
            untraceable: true,
            ..unread()
        };
    }
    if scan.subcommands.is_empty() {
        return unread();
    }
    let mut walk = Walk {
        paths,
        options,
        resolved,
        base: posix_of(&paths.cwd, paths),
        changed_directory: false,
        used_git: false,
    };
    let verdicts: Vec<SubVerdict> = scan
        .commands
        .iter()
        .map(|subcommand| judge(subcommand, &mut walk))
        .collect();
    let combined = combine(&verdicts, &walk, command, !scan.functions.is_empty());
    ShellVerdict {
        untraceable: is_untraceable(&scan, &walk),
        read_only_parts: verdicts.iter().map(|verdict| verdict.read_only).collect(),
        subcommands: scan.subcommands,
        ..combined
    }
}
