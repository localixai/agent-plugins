//! Port of `src/rules/decide/read-only-set.ts`, switched.
//!
//! The built-in read-only Bash commands (permissions.md, "Read-only commands") and what keeps each
//! one's arguments read-only. `cd` and `git` have their own rules in `shell-commands.ts`.

use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;

use crate::js::{Pattern, SPACE_CLASS, regex_test};
use crate::registry::{Function, Set};
use crate::rules::bash::words::{ShellWord, spellings_of};
use crate::rules::path::network::is_network_path;
use crate::rules::types::{PathContext, Platform};

/// "The set includes `ls`, `cat`, `echo`, `pwd`, `head`, `tail`, `grep`, `find`, `wc`, `which`,
/// `diff`, `stat`, `du`, `cd`, and read-only forms of `git`", and the docs' prompt cases name
/// `docker`, `file` and `ps` as members.
pub const READ_ONLY_PROGRAMS: [&str; 20] = [
    "ls", "cat", "echo", "pwd", "head", "tail", "grep", "find", "wc", "which", "diff", "stat",
    "du", "file", "ps", "docker", "podman", "node", "python", "python3",
];

/// `--output <file>` / `--output=<file>`: `git diff`, `log` and `show` write their output to a
/// file.
fn writes_output(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--output" || arg.starts_with("--output="))
}

/// The flags of `git branch` that only list; any other word names, moves or deletes a branch.
const BRANCH_LISTING: [&str; 9] = [
    "--show-current",
    "--list",
    "-a",
    "--all",
    "-r",
    "--remotes",
    "-v",
    "-vv",
    "--verbose",
];

/// What keeps a `git` subcommand's arguments read-only.
pub type GitForm = fn(&[String]) -> bool;

/// The "read-only forms of `git`" the docs do not list: a seed (spec/judgments.md), each
/// subcommand with what keeps its arguments read-only.
pub const GIT_READ_ONLY: [(&str, GitForm); 5] = [
    ("status", |_| true),
    ("log", |args| !writes_output(args)),
    ("diff", |args| !writes_output(args)),
    ("show", |args| !writes_output(args)),
    ("branch", |args| {
        args.iter()
            .all(|arg| BRANCH_LISTING.contains(&arg.as_str()))
    }),
];

/// The read-only forms of `docker` and `podman`: a seed (spec/judgments.md).
const CONTAINER_READ_ONLY: [&str; 6] = ["ps", "images", "logs", "inspect", "version", "info"];

/// "a flag that selects a different daemon, such as `-H`, `--context`, or Podman's `--url` and
/// `--connection`".
static DAEMON_FLAG: Pattern = LazyLock::new(|| {
    Regex::new(r"^(?:-H|-c|--host|--context|--url|--connection)(?:=|$)|^-H.").ok()
});

/// `-m`/`--magic-file` and `-f`/`--files-from` "make `file` open the paths named in the flag's
/// value"; JavaScript's `.` is `[^\n\r\u2028\u2029]`.
static FILE_OPENING_FLAG: Pattern = LazyLock::new(|| {
    Regex::new(
        r"^(?:-[A-Za-z]*[mf][A-Za-z]*|--magic-file(?:=[^\n\r\u{2028}\u{2029}]*)?|--files-from(?:=[^\n\r\u{2028}\u{2029}]*)?)$",
    )
    .ok()
});

/// BSD-style option letters: `aux`, `auxe`.
static BSD_OPTIONS: Pattern = LazyLock::new(|| Regex::new(r"^[A-Za-z]+$").ok());

static DASHED_E: Pattern = LazyLock::new(|| Regex::new(r"^-[A-Za-z]*e").ok());

/// `ps` "prompts when one of its arguments could act as the `e` option": a BSD-style group with
/// an `e` (`auxe`), or a dashed `-e` after a BSD-style group (`aux -e`). `ps aux` and `ps -ef`
/// run.
fn ps_may_print_environment(args: &[String]) -> bool {
    let bsd = args.iter().any(|arg| regex_test(&BSD_OPTIONS, arg));
    args.iter().any(|arg| {
        if arg.starts_with("--") {
            return false;
        }
        if regex_test(&BSD_OPTIONS, arg) {
            return arg.contains('e');
        }
        bsd && regex_test(&DASHED_E, arg)
    })
}

const WHICH_OPTIONS: [&str; 2] = ["-a", "--all"];

fn container_is_read_only(args: &[String]) -> bool {
    let subcommand = args.iter().find(|arg| !arg.starts_with('-'));
    subcommand.is_some_and(|subcommand| CONTAINER_READ_ONLY.contains(&subcommand.as_str()))
        && !args.iter().any(|arg| regex_test(&DAEMON_FLAG, arg))
}

/// A version query alone: `node -v`, `node --version`, `python --version`, `python3 --version`.
/// Judgment (spec/judgments.md, oracle 2.1.319): the docs' set "includes" more than it names;
/// these four forms run without a prompt and every other one asks (`node -V`, `python3 -V`, `npm
/// -v`, `node -v -e 1`).
fn version_query(flags: &[&str], args: &[String]) -> bool {
    matches!(args, [only] if flags.contains(&only.as_str()))
}

/// Per program, whether its arguments keep it read-only: a prompt case of the docs, or a version
/// query. `None` for a program with no entry.
fn argument_check(program: &str, args: &[String]) -> Option<bool> {
    let checked = match program {
        "node" => version_query(&["-v", "--version"], args),
        "python" | "python3" => version_query(&["--version"], args),
        "ps" => !ps_may_print_environment(args),
        "file" => !args.iter().any(|arg| regex_test(&FILE_OPENING_FLAG, arg)),
        "docker" | "podman" => container_is_read_only(args),
        // GNU `which` reads aliases and functions from its input with `-i`, `--read-alias` and
        // `--read-functions`; a plain lookup, with or without `-a`, is all the kit reads as
        // read-only.
        "which" => args
            .iter()
            .all(|arg| !arg.starts_with('-') || WHICH_OPTIONS.contains(&arg.as_str())),
        _ => return None,
    };
    Some(checked)
}

/// Whether the arguments of `program` are read-only: a program with no entry is read-only
/// whatever it is given.
#[must_use]
pub fn arguments_are_read_only(program: &str, words: &[String]) -> bool {
    argument_check(program, words.get(1..).unwrap_or_default()).unwrap_or(true)
}

/// "a command whose arguments include a network (UNC) path ... prompts" (Windows only).
#[must_use]
pub fn has_network_argument(words: &[ShellWord], paths: &PathContext) -> bool {
    paths.platform == Platform::Windows
        && words.get(1..).unwrap_or_default().iter().any(|word| {
            spellings_of(word)
                .iter()
                .any(|spelling| is_network_path(spelling, paths.platform))
        })
}

/// Shell variables whose writes change how the shell runs the rest of the command: "certain
/// special shell variables, such as `PATH` or `IFS`". The docs name two; the rest are the kit's
/// seed (spec/judgments.md).
const SPECIAL_VARIABLES: [&str; 14] = [
    "PATH",
    "IFS",
    "CDPATH",
    "ENV",
    "BASH_ENV",
    "SHELLOPTS",
    "BASHOPTS",
    "PS4",
    "PROMPT_COMMAND",
    "GLOBIGNORE",
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
];

/// JavaScript's `\s`, `\w` and `\b` (no `u` flag) spelt for Rust (RS6).
static SPECIAL_WRITES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let name = format!("(?:{})", SPECIAL_VARIABLES.join("|"));
    let space = format!("[{SPACE_CLASS}]");
    let boundary = format!("(?:^|[{SPACE_CLASS};&|(])");
    let word_boundary = "(?-u:\\b)";
    [
        // `NAME=value`, `export NAME=value`, `declare -x NAME+=value`
        format!(
            r"{boundary}(?:(?:export|declare|typeset|readonly|local){space}+(?:-[A-Za-z0-9_]+{space}+)*)?{name}\+?="
        ),
        // `unset PATH`, `unset -v a PATH`
        format!(r"{boundary}unset{word_boundary}[^;&|\n]*{word_boundary}{name}{word_boundary}"),
        // `for IFS in ...`, `select PATH in ...`, `read IFS`
        format!(r"{boundary}(?:for|select){space}+{name}{word_boundary}"),
        format!(r"{boundary}read{word_boundary}[^;&|\n]*{word_boundary}{name}{word_boundary}"),
    ]
    .iter()
    .filter_map(|source| Regex::new(source).ok())
    .collect()
});

/// Whether a command sets, unsets or loops over a special shell variable, anywhere in it (quoted
/// text included).
#[must_use]
pub fn touches_special_variable(command: &str) -> bool {
    SPECIAL_WRITES
        .iter()
        .any(|pattern| pattern.is_match(command))
}

/// One entry of `GIT_READ_ONLY`, a function the host calls.
struct GitCheck(GitForm);

crate::registry::methods! {
    GitCheck {
        "call" => ok |this, args: Vec<String>| (this.0)(&args);
    }
}

crate::registry::exports! {
    "rules/decide/read-only-set" {
        "READ_ONLY_PROGRAMS" => ok || Set { members: READ_ONLY_PROGRAMS.to_vec() };
        "GIT_READ_ONLY" => ok || GIT_READ_ONLY
            .map(|(name, form)| (name, Function::new(GitCheck(form))))
            .into_iter()
            .collect::<IndexMap<_, _>>();
        "argumentsAreReadOnly" => ok |program: String, words: Vec<String>| arguments_are_read_only(&program, &words);
        "hasNetworkArgument" => ok |words: Vec<ShellWord>, paths: PathContext| has_network_argument(&words, &paths);
        "touchesSpecialVariable" => ok |command: String| touches_special_variable(&command);
    }
}

#[cfg(test)]
mod tests {
    use super::{SPECIAL_WRITES, touches_special_variable};

    #[test]
    fn every_special_write_compiles() {
        assert_eq!(SPECIAL_WRITES.len(), 4);
        assert!(touches_special_variable("export PATH=/x"));
        assert!(!touches_special_variable("echo $PATH"));
    }
}
