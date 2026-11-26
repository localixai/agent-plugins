//! Port of `src/rules/decide/powershell-removal.ts`, switched.
//!
//! permission-modes.md, "Remove-Item in PowerShell": the two cases Claude Code "denies ... in
//! every mode, without asking you" (`bypassPermissions` included: "The Remove-Item in PowerShell
//! denies also apply in this mode"):
//! - `Remove-Item` on a system path ("the filesystem root and its top-level directories, drive
//!   roots and their top-level directories, and your home directory"), or on a wildcard ("a bare
//!   `*`, or any target ending in `/*` or `\*`, including a glob under a shell variable such as
//!   `$dir/*`");
//! - `rd`, `rmdir`, `del` and `erase` run through `cmd` on a system path, where "a PowerShell
//!   variable that follows literal text" counts as empty and "a trailing wildcard counts as the
//!   folder it empties". `CLAUDE_CODE_DISABLE_POWERSHELL_CMD_RM_DENY=1` turns this case off
//!   (`DecideContext.powershellCmdRemovalDenyOff`, the host's reading of the launch environment).

use std::sync::LazyLock;

use regex::Regex;

use super::critical_removal::is_system_directory;
use crate::js::{Pattern, is_whitespace};
use crate::rules::bash::dialect::POWERSHELL_DIALECT;
use crate::rules::bash::powershell::cmdlet_call;
use crate::rules::bash::split::scan_command;
use crate::rules::bash::words::{ShellCommand, ShellWord};
use crate::rules::path::posix::{HostPaths, absolute_path};
use crate::rules::types::PathContext;

/// The named parameters of `Remove-Item` whose value is a target.
const PATH_PARAMETERS: [&str; 4] = ["-path", "-literalpath", "-lp", "-pspath"];
/// Named parameters that take a value that is not a target.
const VALUE_PARAMETERS: [&str; 5] = ["-filter", "-include", "-exclude", "-credential", "-stream"];

/// The words a `Remove-Item` call removes: its positional arguments and its path parameters'
/// values.
fn remove_item_targets(args: &[ShellWord]) -> Vec<&ShellWord> {
    let mut targets = Vec::new();
    let mut at = 0;
    while let Some(word) = args.get(at) {
        let name = word.value.to_lowercase();
        if !name.starts_with('-') {
            targets.push(word);
        } else if PATH_PARAMETERS.contains(&name.as_str())
            || VALUE_PARAMETERS.contains(&name.as_str())
        {
            if let Some(value) = args
                .get(at + 1)
                .filter(|_| PATH_PARAMETERS.contains(&name.as_str()))
            {
                targets.push(value);
            }
            at += 1;
        }
        at += 1;
    }
    targets
}

/// `/[/\\]\*$/`.
fn ends_in_folder_wildcard(value: &str) -> bool {
    value.ends_with("/*") || value.ends_with("\\*")
}

fn is_wildcard(value: &str) -> bool {
    value == "*" || ends_in_folder_wildcard(value)
}

fn system_path(text: &str, paths: &PathContext) -> bool {
    !text.is_empty() && is_system_directory(&absolute_path(text, &HostPaths::from(paths)), paths)
}

/// A `Remove-Item` target the docs deny: a wildcard, or a literal system path.
fn remove_item_denied(word: &ShellWord, paths: &PathContext) -> bool {
    is_wildcard(&word.value) || (!word.expands && !word.globs && system_path(&word.value, paths))
}

const CMD_NAMES: [&str; 2] = ["cmd", "cmd.exe"];
const CMD_RUN_SWITCHES: [&str; 2] = ["/c", "/k"];
const CMD_REMOVALS: [&str; 4] = ["rd", "rmdir", "del", "erase"];

/// What `cmd` runs after `/c` or `/k`, one word per token (a quoted command is split again, at
/// JavaScript's `\s`).
fn cmd_tokens(args: &[ShellWord]) -> Vec<String> {
    let Some(run) = args
        .iter()
        .position(|word| CMD_RUN_SWITCHES.contains(&word.value.to_lowercase().as_str()))
    else {
        return Vec::new();
    };
    args.get(run + 1..)
        .unwrap_or_default()
        .iter()
        .flat_map(|word| {
            word.value
                .split(is_whitespace)
                .filter(|token| !token.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// `/\$(?:\{[^}]*\}|[\w:]+)/g`: a PowerShell variable.
static VARIABLE: Pattern = LazyLock::new(|| Regex::new(r"\$(?:\{[^}]*\}|[A-Za-z0-9_:]+)").ok());

/// A `cmd` target as judged: a variable after literal text is empty, a trailing wildcard is its
/// folder.
fn cmd_target(token: &str) -> String {
    let bare = VARIABLE.as_ref().map_or_else(
        || token.to_owned(),
        |variable| variable.replace_all(token, "").into_owned(),
    );
    let folder = if ends_in_folder_wildcard(&bare) {
        format!("{}\\", bare.get(..bare.len() - 2).unwrap_or_default())
    } else {
        bare
    };
    if folder == "*" {
        ".".to_owned()
    } else {
        folder
    }
}

fn cmd_removal_denied(args: &[ShellWord], paths: &PathContext) -> bool {
    let tokens = cmd_tokens(args);
    let Some((verb, rest)) = tokens.split_first() else {
        return false;
    };
    if !CMD_REMOVALS.contains(&verb.to_lowercase().as_str()) {
        return false;
    }
    rest.iter()
        .filter(|token| !token.starts_with('/'))
        .any(|token| system_path(&cmd_target(token), paths))
}

fn denied_command(command: &ShellCommand, paths: &PathContext, cmd_off: bool) -> bool {
    let Some(found) = cmdlet_call(command) else {
        return false;
    };
    let cmdlet = found.cmdlet.to_lowercase();
    if cmdlet == "remove-item" {
        return remove_item_targets(&found.args)
            .iter()
            .any(|word| remove_item_denied(word, paths));
    }
    !cmd_off && CMD_NAMES.contains(&cmdlet.as_str()) && cmd_removal_denied(&found.args, paths)
}

/// Whether a PowerShell command holds a removal the docs deny in every mode.
#[must_use]
pub fn powershell_removal_denied(command: &str, paths: &PathContext, cmd_check_off: bool) -> bool {
    scan_command(command, Some(&POWERSHELL_DIALECT))
        .commands
        .iter()
        .any(|each| denied_command(each, paths, cmd_check_off))
}

crate::registry::exports! {
    "rules/decide/powershell-removal" {
        "powershellRemovalDenied" => ok |command: String, paths: PathContext, cmd_check_off: bool| powershell_removal_denied(&command, &paths, cmd_check_off);
    }
}
