//! Ported from `src/rules/decide/critical-removal.ts`; only the core uses it now, so it has no wrapper.
//!
//! permission-modes.md, "Critical paths": an `rm` or `rmdir` whose target Claude Code "recognizes
//! as a critical path": the filesystem root, a top-level directory, the home directory, the
//! working directory and its parents, and (only as a glob under them) the additional working
//! directories and their parents. The table "Other targets that count as critical paths" adds the
//! shell-variable forms and command substitution. The docs say the check "can miss some commands":
//! what it does not recognize gets no special handling here either.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::scope::posix_of;
use crate::js::{Pattern, SPACE_CLASS, regex_test};
use crate::rules::bash::split::scan_command;
use crate::rules::bash::words::{ShellCommand, ShellWord, spellings_of};
use crate::rules::bash::wrappers::stripped_words;
use crate::rules::path::posix::{HostPaths, absolute_path, dirname_of, relative_within};
use crate::rules::types::{PathContext, PermissionBehavior, Platform};

const REMOVERS: [&str; 2] = ["rm", "rmdir"];
const SHELLS: [&str; 5] = ["sh", "bash", "zsh", "dash", "ksh"];

/// The words after which a command runs another: `find -exec sh -c '...'` (the docs' own example).
const EXECUTORS: [&str; 4] = ["-exec", "-execdir", "-ok", "-okdir"];

/// "A shell variable followed by one common top-level directory name, such as `mnt`, `tmp`, `usr`,
/// or `Users`."
const TOP_LEVEL_NAMES: &str = "mnt|tmp|usr|Users";

const VARIABLE: &str = r"\$(?:\{[A-Za-z_][A-Za-z0-9_]*\}|[A-Za-z_][A-Za-z0-9_]*|[0-9@*])";
static UNDER_VARIABLE: Pattern =
    LazyLock::new(|| Regex::new(&format!(r"^{VARIABLE}(?:/\*|/)$")).ok());
static VARIABLE_TOP_LEVEL: Pattern =
    LazyLock::new(|| Regex::new(&format!(r"^{VARIABLE}/(?:{TOP_LEVEL_NAMES})$")).ok());
static GUARDED: Pattern =
    LazyLock::new(|| Regex::new(r"^\$\{[A-Za-z_][A-Za-z0-9_]*:\?[^}]*\}").ok());
static WHOLE_SUBSTITUTION: Pattern =
    LazyLock::new(|| Regex::new(r"^(?:\$\([^)]*\)|`[^`]*`)$").ok());
static TRAILING_SUBSTITUTION: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:\$\([^)]*\)|`[^`]*`)$").ok());
static MANY_LEVELS: Pattern = LazyLock::new(|| Regex::new(r"/\*/(?:\*)?$").ok());
static POSITIONAL: Pattern = LazyLock::new(|| Regex::new(r"^\$(?:\{[0-9@*]+\}|[0-9@*])").ok());
static ONLY_BACKSLASHES: Pattern = LazyLock::new(|| Regex::new(r"^\\+$").ok());
/// `/^\\\\\?\\Volume\{[^}]*\}/i`: the flag folds the letters of `Volume` only.
static VOLUME_GUID: Pattern = LazyLock::new(|| Regex::new(r"^\\\\\?\\(?i:volume)\{[^}]*\}").ok());
/// `/^[^/~$\\].*\/\*$/`, `.` being JavaScript's (no line terminator).
static RELATIVE_GLOB: Pattern =
    LazyLock::new(|| Regex::new(r"^[^/~$\\][^\n\r\u{2028}\u{2029}]*/\*$").ok());
static DIRECTORY_ASSIGNMENT: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?:^|[{SPACE_CLASS};&|(])([A-Za-z_][A-Za-z0-9_]*)=(?:\$\((?:pwd|git rev-parse --show-toplevel)\)|`pwd`)"
    ))
    .ok()
});
static ASSIGNED_VARIABLE: Pattern =
    LazyLock::new(|| Regex::new(r"^\$\{?([A-Za-z_][A-Za-z0-9_]*)\}?$").ok());
static RECURSIVE_OPTION: Pattern =
    LazyLock::new(|| Regex::new(r"^(?:-[A-Za-z]*[rR][A-Za-z]*|--recursive)$").ok());

/// The variables the command itself assigns from a directory-printing substitution.
fn assigned_directories(command: &str) -> HashSet<String> {
    DIRECTORY_ASSIGNMENT
        .as_ref()
        .map(|pattern| {
            pattern
                .captures_iter(command)
                .map(|found| found.get(1).map_or("", |name| name.as_str()).to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone)]
struct Scope<'a> {
    paths: &'a PathContext,
    assigned: HashSet<String>,
    recursive: bool,
    /// Arguments after an inline script give `$1` and `$@` a value.
    bound: bool,
    /// A `cd` ran earlier in the command, so a relative target no longer says where it is.
    moved: bool,
}

/// On Windows, "its 8.3 short name counts too, such as `C:\Users\LONGNA~1`": a `~N` name beside
/// the home directory.
fn is_home_short_name(directory: &str, paths: &PathContext) -> bool {
    let home = posix_of(&paths.home, paths);
    paths.platform == Platform::Windows
        && dirname_of(directory) == dirname_of(&home)
        && ends_with_short_number(directory)
}

/// `/~\d+$/`.
fn ends_with_short_number(text: &str) -> bool {
    let digits = text.trim_end_matches(|character: char| character.is_ascii_digit());
    digits.len() < text.len() && digits.ends_with('~')
}

/// The filesystem root, a top-level directory (a drive root or its children on Windows) or the
/// home directory.
#[must_use]
pub fn is_system_directory(directory: &str, paths: &PathContext) -> bool {
    let depth = directory
        .split('/')
        .filter(|segment| !segment.is_empty())
        .count();
    let top_level = if paths.platform == Platform::Windows {
        2
    } else {
        1
    };
    depth <= top_level
        || relative_within(
            &posix_of(&paths.home, paths),
            directory,
            Some(paths.platform),
        )
        .as_deref()
            == Some("")
        || is_home_short_name(directory, paths)
}

/// Whether `directory` is critical: a system directory, the working directory or a parent of it.
#[must_use]
pub fn is_critical_directory(directory: &str, paths: &PathContext) -> bool {
    is_system_directory(directory, paths)
        || relative_within(
            directory,
            &posix_of(&paths.cwd, paths),
            Some(paths.platform),
        )
        .is_some()
}

/// "Spellings such as `\\?\C:\` and `\\localhost\C$` count as `C:\`": `/^\\\\\?\\(?=[A-Za-z]:)/`
/// is dropped, and `/^\\\\localhost\\([A-Za-z])\$/i` is the drive.
fn windows_spelling(target: &str, platform: Platform) -> String {
    if platform != Platform::Windows {
        return target.to_owned();
    }
    let drive = |text: &str| {
        let mut characters = text.chars();
        characters
            .next()
            .is_some_and(|letter| letter.is_ascii_alphabetic())
            && characters.next() == Some(':')
    };
    let unprefixed = match target.strip_prefix(r"\\?\") {
        Some(rest) if drive(rest) => rest,
        _ => target,
    };
    let prefix = r"\\localhost\";
    let local = unprefixed
        .get(..prefix.len())
        .filter(|start| start.eq_ignore_ascii_case(prefix))
        .and_then(|_| unprefixed.get(prefix.len()..))
        .and_then(|rest| {
            let mut characters = rest.chars();
            let letter = characters.next().filter(char::is_ascii_alphabetic)?;
            characters
                .as_str()
                .strip_prefix('$')
                .map(|after| format!("{letter}:{after}"))
        });
    local.unwrap_or_else(|| unprefixed.to_owned())
}

/// Whether `directory` is, or is a parent of, an additional working directory.
fn covers_additional_directory(directory: &str, paths: &PathContext) -> bool {
    paths.additional_directories.iter().any(|added| {
        relative_within(directory, &posix_of(added, paths), Some(paths.platform)).is_some()
    })
}

/// A literal path target, as a glob under a directory or as the path itself.
fn is_critical_path(target: &str, paths: &PathContext) -> bool {
    let glob = target.ends_with("/*");
    let text = match target.strip_suffix("/*") {
        Some("") => "/",
        Some(stem) => stem,
        None => target,
    };
    let resolved = absolute_path(text, &HostPaths::from(paths));
    if is_critical_directory(&resolved, paths) {
        return true;
    }
    glob && covers_additional_directory(&resolved, paths)
}

fn is_assigned_variable(target: &str, assigned: &HashSet<String>) -> bool {
    ASSIGNED_VARIABLE
        .as_ref()
        .and_then(|pattern| pattern.captures(target))
        .and_then(|found| found.get(1))
        .is_some_and(|name| assigned.contains(name.as_str()))
}

type ShellForm = fn(&str, &Scope<'_>) -> bool;

/// The forms of "Other targets that count as critical paths" that need no path arithmetic.
const SHELL_FORMS: [ShellForm; 6] = [
    // "The same form under a positional parameter ... when nothing in the command gives it a value".
    |target, scope| {
        (regex_test(&UNDER_VARIABLE, target) || regex_test(&VARIABLE_TOP_LEVEL, target))
            && !(scope.bound && regex_test(&POSITIONAL, target))
    },
    |target, scope| is_assigned_variable(target, &scope.assigned),
    |target, scope| scope.recursive && regex_test(&WHOLE_SUBSTITUTION, target),
    |target, _| regex_test(&MANY_LEVELS, target),
    |target, _| regex_test(&ONLY_BACKSLASHES, target) || regex_test(&VOLUME_GUID, target),
    // `cd logs && rm -rf a/*`: after a `cd` a relative glob reaches a directory the check cannot place.
    |target, scope| scope.moved && regex_test(&RELATIVE_GLOB, target),
];

/// `target.replace(/^\$\{?HOME\}?(?=\/|$)/, '~')`.
fn replace_home(target: &str) -> String {
    let opened = target
        .strip_prefix("${")
        .or_else(|| target.strip_prefix('$'));
    let Some(after) = opened.and_then(|rest| rest.strip_prefix("HOME")) else {
        return target.to_owned();
    };
    let ends = |rest: &str| rest.is_empty() || rest.starts_with('/');
    match after.strip_prefix('}') {
        Some(rest) if ends(rest) => format!("~{rest}"),
        _ if ends(after) => format!("~{after}"),
        _ => target.to_owned(),
    }
}

/// `target.replace(TRAILING_SUBSTITUTION, '')`: the first match, which ends the text.
fn trailing_substitution_removed(target: &str) -> String {
    TRAILING_SUBSTITUTION.as_ref().map_or_else(
        || target.to_owned(),
        |pattern| pattern.replace(target, "").into_owned(),
    )
}

/// Whether one operand of an `rm` or `rmdir` is a critical-path target, in either of its
/// spellings.
fn is_critical_target(operand: &ShellWord, scope: &Scope<'_>) -> bool {
    spellings_of(operand)
        .iter()
        .any(|spelling| is_critical_spelling(spelling, scope))
}

fn is_critical_spelling(spelling: &str, scope: &Scope<'_>) -> bool {
    let target = windows_spelling(spelling, scope.paths.platform);
    if regex_test(&GUARDED, &target) {
        return false;
    }
    if SHELL_FORMS.iter().any(|form| form(&target, scope)) {
        return true;
    }
    let without_substitution = trailing_substitution_removed(&target);
    let literal = replace_home(&without_substitution);
    !literal.contains('$') && !literal.is_empty() && is_critical_path(&literal, scope.paths)
}

/// The operands of an `rm` or `rmdir` and whether it is recursive: options end at `--`.
fn operands_of(words: &[ShellWord]) -> (Vec<&ShellWord>, bool) {
    let mut operands = Vec::new();
    let mut recursive = false;
    let mut options = true;
    for word in words {
        let value = word.value.as_str();
        if options && value == "--" {
            options = false;
        } else if options && value.starts_with('-') {
            recursive = recursive || regex_test(&RECURSIVE_OPTION, value);
        } else {
            operands.push(word);
        }
    }
    (operands, recursive)
}

/// An inline shell script, as the inner shell receives it, and whether arguments after it bind
/// `$1`.
struct InlineScript {
    script: String,
    bound: bool,
}

/// `/^(["'])([\s\S]*)\1$/`: the quote and the text inside it.
fn quoted(text: &str) -> Option<(char, &str)> {
    let quote = text
        .chars()
        .next()
        .filter(|first| matches!(first, '"' | '\''))?;
    let inner = text.get(1..)?.strip_suffix(quote)?;
    Some((quote, inner))
}

static DOUBLE_QUOTED_EXPANSION: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"\\([\\"$`])|\$(?:\{[0-9@*]+\}|[0-9@*])"#).ok());

/// "When the script is double-quoted, the invoking shell expands its variables before the inner
/// shell receives the script": a positional parameter of the invoking shell is empty, and `\"` is a
/// quote.
fn expand_double_quoted(text: &str) -> String {
    DOUBLE_QUOTED_EXPANSION.as_ref().map_or_else(
        || text.to_owned(),
        |pattern| {
            pattern
                .replace_all(text, |found: &regex::Captures<'_>| {
                    found
                        .get(1)
                        .map_or_else(String::new, |escaped| escaped.as_str().to_owned())
                })
                .into_owned()
        },
    )
}

/// The script a `sh -c '...'` runs, as the inner shell sees it, or `None`.
fn inline_script(words: &[ShellWord]) -> Option<InlineScript> {
    let at = words.iter().position(|word| word.value == "-c")?;
    let word = words.get(at + 1)?;
    let script = match quoted(&word.text) {
        Some(('"', inner)) => expand_double_quoted(inner),
        _ => word.value.clone(),
    };
    Some(InlineScript {
        script,
        bound: words.len() > at + 2,
    })
}

/// The shells a subcommand runs with `-c`: as its program, or after `-exec` and the like.
fn inline_scripts(words: &[ShellWord]) -> Vec<InlineScript> {
    words
        .iter()
        .enumerate()
        .filter_map(|(index, word)| {
            let after_executor = index
                .checked_sub(1)
                .and_then(|before| words.get(before))
                .is_some_and(|before| EXECUTORS.contains(&before.value.as_str()));
            let runs = SHELLS.contains(&word.value.as_str()) && (index == 0 || after_executor);
            if runs {
                inline_script(words.get(index + 1..).unwrap_or_default())
            } else {
                None
            }
        })
        .collect()
}

/// Whether one subcommand removes a critical path: an `rm` or `rmdir`, or an inline shell script
/// that does.
fn subcommand_removes(subcommand: &ShellCommand, command: &Scope<'_>, depth: u32) -> bool {
    let words = stripped_words(subcommand, PermissionBehavior::Deny, &HashSet::new());
    let Some((program, rest)) = words.split_first() else {
        return false;
    };
    // `"rm"`, `\rm` and `$'rm'` run `rm` all the same
    // (crates/agent-plugins/src/rules/bash/unquote.rs).
    if REMOVERS.contains(&program.value.as_str()) {
        let (operands, recursive) = operands_of(rest);
        let scope = Scope {
            recursive,
            ..command.clone()
        };
        return operands
            .iter()
            .any(|operand| is_critical_target(operand, &scope));
    }
    depth < 4
        && inline_scripts(&words).iter().any(|inline| {
            removes_critical_path(&inline.script, command.paths, depth + 1, inline.bound)
        })
}

/// "Dangerous rm operation not ruled out": a command the kit cannot read to the end may hide a
/// removal.
#[must_use]
pub fn cannot_rule_out_removal(command: &str) -> bool {
    !scan_command(command, None).parseable
}

fn changes_directory(subcommand: &ShellCommand) -> bool {
    subcommand
        .words
        .first()
        .is_some_and(|word| word.value == "cd")
}

/// Whether a shell command contains a recognized critical-path removal, anywhere in it: in a
/// compound command, a subshell, a command substitution or an inline `sh -c` script.
#[must_use]
pub fn removes_critical_path(command: &str, paths: &PathContext, depth: u32, bound: bool) -> bool {
    let mut scope = Scope {
        paths,
        assigned: assigned_directories(command),
        recursive: false,
        bound,
        moved: false,
    };
    for sub in &scan_command(command, None).commands {
        if subcommand_removes(sub, &scope, depth) {
            return true;
        }
        scope.moved = scope.moved || changes_directory(sub);
    }
    false
}
