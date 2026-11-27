//! Ported from `src/rules/decide/powershell-edits.ts`; only the core uses it now, so it has no wrapper.
//!
//! permission-modes.md, "Auto-approve file edits with acceptEdits mode": "When the PowerShell tool
//! is enabled, `acceptEdits` mode also auto-approves `Set-Content`, `Add-Content`,
//! `Clear-Content`, and `Remove-Item` on in-scope paths, along with their common aliases." The
//! aliases are the rules' own (`crates/agent-plugins/src/rules/bash/powershell_aliases.rs`), and so
//! is the reading of the name (`&`, a module qualifier).

use std::sync::LazyLock;

use regex::Regex;

use super::context::ProtectedPathOptions;
use super::critical_removal::{is_critical_directory, is_system_directory};
use super::scope::posix_of;
use super::shell_commands::{ShellVerdict, SubVerdict, judge_write, unread};
use super::shell_words::{Walk, operand_path};
use crate::js::Pattern;
use crate::rules::bash::dialect::POWERSHELL_DIALECT;
use crate::rules::bash::powershell::cmdlet_call;
use crate::rules::bash::split::scan_command;
use crate::rules::bash::words::{ShellCommand, ShellWord};
use crate::rules::types::PathContext;

/// How many positional arguments a cmdlet takes after its path: `Set-Content` and `Add-Content`
/// take the content.
const CONTENT_ARGUMENTS: [(&str, usize); 4] = [
    ("set-content", 1),
    ("add-content", 1),
    ("clear-content", 0),
    ("remove-item", 0),
];

fn content_arguments(cmdlet: &str) -> Option<usize> {
    CONTENT_ARGUMENTS
        .iter()
        .find(|(name, _)| *name == cmdlet)
        .map(|(_, count)| *count)
}

/// The named parameters the kit validates; any other named parameter makes a call not approved.
#[derive(Clone, Copy)]
enum ParameterKind {
    Path,
    Value,
    Switch,
    Recurse,
}

const PARAMETERS: [(&str, ParameterKind); 8] = [
    ("-path", ParameterKind::Path),
    ("-literalpath", ParameterKind::Path),
    ("-value", ParameterKind::Value),
    ("-encoding", ParameterKind::Value),
    ("-force", ParameterKind::Switch),
    ("-nonewline", ParameterKind::Switch),
    ("-whatif", ParameterKind::Switch),
    ("-confirm", ParameterKind::Switch),
];

/// `-Recurse` and its unambiguous prefixes, for `Remove-Item` only.
static RECURSE: Pattern =
    LazyLock::new(|| Regex::new(r"(?i)^-r(?:e(?:c(?:u(?:r(?:s(?:e)?)?)?)?)?)?$").ok());

struct Reading<'a> {
    paths: Vec<&'a ShellWord>,
    positional: Vec<&'a ShellWord>,
    recursive: bool,
    plain: bool,
}

/// What each kind of named parameter does to the reading, and how many words after it are its
/// value.
fn handle<'a>(
    kind: ParameterKind,
    reading: &mut Reading<'a>,
    next: Option<&'a ShellWord>,
) -> usize {
    match kind {
        ParameterKind::Path => {
            match next {
                None => reading.plain = false,
                Some(next) => reading.paths.push(next),
            }
            1
        }
        ParameterKind::Value => 1,
        ParameterKind::Switch => 0,
        ParameterKind::Recurse => {
            reading.recursive = true;
            0
        }
    }
}

fn parameter_kind(cmdlet: &str, word: &str) -> Option<ParameterKind> {
    if cmdlet == "remove-item"
        && RECURSE
            .as_ref()
            .is_some_and(|recurse| recurse.is_match(word))
    {
        return Some(ParameterKind::Recurse);
    }
    let lower = word.to_lowercase();
    PARAMETERS
        .iter()
        .find(|(name, _)| *name == lower)
        .map(|(_, kind)| *kind)
}

/// A cmdlet call, parsed: the paths it names, whether it is recursive and whether anything else
/// is in it.
struct Call<'a> {
    cmdlet: String,
    paths: Vec<&'a ShellWord>,
    recursive: bool,
    plain: bool,
}

/// "A positional argument that contains a quote character ... still prompts": its quoted and
/// unquoted readings differ. The quotes that wrap the whole argument do not count
/// (`text.replace(/^(["'])([\s\S]*)\1$/, '$2')`).
fn has_quote_character(word: &ShellWord) -> bool {
    let text = word.text.as_str();
    let inner = text
        .chars()
        .next()
        .filter(|first| matches!(first, '"' | '\''))
        .filter(|first| text.len() >= 2 && text.ends_with(*first))
        .and_then(|_| text.get(1..text.len() - 1))
        .unwrap_or(text);
    inner.contains(['"', '\''])
}

/// Whether a reading holds only what the kit validates: a path, the right number of arguments, no
/// quote character.
fn is_plain(reading: &Reading<'_>, cmdlet: &str, content: usize) -> bool {
    reading.plain
        && !reading.paths.is_empty()
        && content <= content_arguments(cmdlet).unwrap_or(0)
        && !reading
            .positional
            .iter()
            .any(|word| has_quote_character(word))
}

/// Reads the words after the cmdlet name; `plain` is false for anything the kit cannot validate.
fn read_call<'a>(cmdlet: &str, words: &'a [ShellWord]) -> Call<'a> {
    let mut reading = Reading {
        paths: Vec::new(),
        positional: Vec::new(),
        recursive: false,
        plain: true,
    };
    let mut at = 0;
    while let Some(word) = words.get(at) {
        let named = word.value.starts_with('-');
        let kind = if named {
            parameter_kind(cmdlet, &word.value)
        } else {
            None
        };
        if !named {
            reading.positional.push(word);
        } else if let Some(kind) = kind {
            at += handle(kind, &mut reading, words.get(at + 1));
        } else {
            reading.plain = false;
        }
        at += 1;
    }
    // With no `-Path`, the first positional argument is the path; any other is content.
    let first = reading.positional.first().copied();
    let positional_path = reading.paths.is_empty() && first.is_some();
    if let Some(first) = first.filter(|_| positional_path) {
        reading.paths.push(first);
    }
    let content = reading.positional.len() - usize::from(positional_path);
    let plain = is_plain(&reading, cmdlet, content);
    Call {
        cmdlet: cmdlet.to_owned(),
        paths: reading.paths,
        recursive: reading.recursive,
        plain,
    }
}

/// `Remove-Item` "gets its own check" (permission-modes.md, "Remove-Item in PowerShell"): a system
/// path or a wildcard is denied in every mode (`powershell_removal.rs`, a gate of `decide`), and
/// the working directory or a parent of it with `-Recurse` needs the mode's approval, so
/// `acceptEdits` does not approve it (spec/judgments.md).
fn removal_is_ordinary(call: &Call<'_>, targets: &[&str], paths: &PathContext) -> bool {
    if call.cmdlet != "remove-item" {
        return true;
    }
    targets.iter().all(|target| {
        if call.recursive {
            !is_critical_directory(target, paths)
        } else {
            !is_system_directory(target, paths)
        }
    })
}

fn judge_cmdlet(command: &ShellCommand, walk: &Walk<'_>) -> Option<SubVerdict> {
    let found = cmdlet_call(command)?;
    let cmdlet = found.cmdlet.to_lowercase();
    content_arguments(&cmdlet)?;
    let call = read_call(&cmdlet, &found.args);
    if !call.plain {
        return None;
    }
    let targets: Vec<Option<String>> = call
        .paths
        .iter()
        .map(|path| operand_path(path, walk))
        .collect();
    let known: Vec<&str> = targets.iter().flatten().map(String::as_str).collect();
    if !removal_is_ordinary(&call, &known, walk.paths) {
        return None;
    }
    Some(judge_write(&targets, walk))
}

/// PowerShell, as far as the modes need it: split into subcommands for the rules, with the four
/// `acceptEdits` cmdlets judged on their paths. PowerShell has no read-only set in `decide`.
#[must_use]
pub fn powershell_verdict(
    command: &str,
    paths: &PathContext,
    options: &ProtectedPathOptions,
    resolved: &[String],
) -> ShellVerdict {
    let scan = scan_command(command, Some(&POWERSHELL_DIALECT));
    if !scan.parseable || scan.subcommands.is_empty() {
        return unread();
    }
    let walk = Walk {
        paths,
        options,
        resolved,
        base: posix_of(&paths.cwd, paths),
        changed_directory: false,
        used_git: false,
    };
    let judged: Vec<Option<SubVerdict>> = scan
        .commands
        .iter()
        .map(|subcommand| judge_cmdlet(subcommand, &walk))
        .collect();
    ShellVerdict {
        edits_in_scope: judged
            .iter()
            .all(|verdict| verdict.is_some_and(|v| v.edits)),
        writes_protected: judged
            .iter()
            .any(|verdict| verdict.is_some_and(|v| v.protected_write)),
        writes_protected_via_link: judged
            .iter()
            .any(|verdict| verdict.is_some_and(|v| v.via_link)),
        read_only_parts: vec![false; scan.subcommands.len()],
        subcommands: scan.subcommands,
        ..unread()
    }
}
