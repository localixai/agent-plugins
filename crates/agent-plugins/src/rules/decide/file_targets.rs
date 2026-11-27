//! Port of `src/rules/decide/file-targets.ts`, switched.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use super::redirects::{Target, TargetKind, TargetSource, literal_of, redirect_targets};
use crate::js::Pattern;
use crate::rules::bash::split::scan_command;
use crate::rules::bash::words::ShellWord;
use crate::rules::bash::wrappers::stripped_words;
use crate::rules::types::{PathContext, PermissionBehavior};

/// The files a Bash command names, as the `Read` and `Edit` rules see them (permissions.md, "What
/// `Read` and `Edit` rules cover": the rules apply "to file commands Claude Code recognizes in
/// Bash, such as `cat`, `head`, `tail`, `sed`, and `tee`, and to the targets of Bash redirections
/// such as `> file` and `< file`"; "a command that reads files without naming them" is not
/// covered). A lexical reading of the line, not a sandbox; an operand a variable or substitution
/// hides is left out. Judgment (spec/judgments.md): the docs name five of the commands, so the set
/// below is the kit's, and a `cd` earlier in the line does not move the directory later operands
/// are read against.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BashFileTargets {
    /// Absolute POSIX paths the command reads.
    pub reads: Vec<String>,
    /// Absolute POSIX paths the command writes.
    pub writes: Vec<String>,
}

/// The files one command reads and writes by naming them, and whether Claude Code checks them.
pub struct Operands<'a> {
    /// `reads`.
    pub reads: Vec<&'a ShellWord>,
    /// `writes`.
    pub writes: Vec<&'a ShellWord>,
    /// Claude Code checks the paths it names against the working directories.
    pub checked: bool,
}

/// What a program does with the file operands it is given.
#[derive(Clone, Copy)]
enum Role {
    Reads,
    Writes,
    Copies,
    Edits,
}

fn role_of<'a>(
    role: Role,
    files: Vec<&'a ShellWord>,
    args: &[ShellWord],
) -> (Vec<&'a ShellWord>, Vec<&'a ShellWord>) {
    match role {
        Role::Reads => (files, Vec::new()),
        Role::Writes => (Vec::new(), files),
        Role::Copies => {
            if files.len() < 2 {
                return (Vec::new(), Vec::new());
            }
            let mut reads = files;
            let last = reads.pop().into_iter().collect();
            (reads, last)
        }
        Role::Edits => {
            let in_place = args
                .iter()
                .any(|word| word.value.starts_with("-i") || word.value.starts_with("--in-place"));
            role_of(
                if in_place { Role::Writes } else { Role::Reads },
                files,
                args,
            )
        }
    }
}

/// The file commands Claude Code 2.1.322 path-checked: a file they name outside the working
/// directories was asked despite a prefix rule (spec/judgments.md).
const READING: [&str; 2] = [
    "cat head tail nl wc sort uniq diff cmp od hexdump strings file stat md5sum sha256sum base64",
    "jq awk cut tr column grep rg egrep fgrep ls find tac rev fold paste comm expand",
];
const WRITING: &str = "touch rm mkdir rmdir";
const COPYING: &str = "cp mv";
/// Commands that name files Claude Code ran outside the working directories under their prefix
/// rule, with no path check: the kit applies `Read` and `Edit` deny and ask rules to them all the
/// same (docs/SECURITY.md), and nothing else.
const UNCHECKED_READING: &str = "less more bat xxd shasum source . yq du";
const UNCHECKED_WRITING: &str = "truncate chmod chown";
const UNCHECKED_COPYING: &str = "install ln rsync";

/// `PROGRAMS.get(name)`: the role and whether it is checked. `tee` is not here: the redirection
/// reader already counts its files as writes.
fn program_of(name: &str) -> Option<(Role, bool)> {
    let lists: [(&[&str], Role, bool); 7] = [
        (&READING, Role::Reads, true),
        (&[WRITING], Role::Writes, true),
        (&[COPYING], Role::Copies, true),
        (&["sed"], Role::Edits, true),
        (&[UNCHECKED_READING], Role::Reads, false),
        (&[UNCHECKED_WRITING], Role::Writes, false),
        (&[UNCHECKED_COPYING], Role::Copies, false),
    ];
    // A later list wins, as a later entry of a `Map` built from the lists does.
    lists
        .iter()
        .rev()
        .find(|(names, _, _)| names.iter().any(|line| line.split(' ').any(|n| n == name)))
        .map(|(_, role, checked)| (*role, *checked))
}

/// The first operand is a pattern or a program, not a file, unless an option gave it.
const PATTERN_FIRST: [&str; 10] = [
    "grep", "rg", "egrep", "fgrep", "jq", "yq", "awk", "sed", "tr", "cut",
];
const PATTERN_OPTIONS: [&str; 3] = ["-e", "--regexp", "--expression"];
static GIVES_PATTERN: Pattern = LazyLock::new(|| {
    Regex::new(r"(?s)^(?:-[A-Za-z]*[ef].*|--(?:regexp|file|expression|from-file)(?:=.*)?)$").ok()
});

fn gives_pattern(text: &str) -> bool {
    GIVES_PATTERN
        .as_ref()
        .is_some_and(|pattern| pattern.is_match(text))
}

/// The files one command, its wrappers taken off, reads and writes by naming them; `None` for a
/// program the kit does not know as a file command. The read-only and read-block checks read the
/// same operands (`shell_commands.rs`).
#[must_use]
pub fn file_operands(words: &[ShellWord]) -> Option<Operands<'_>> {
    let (program, args) = words.split_first()?;
    let (role, checked) = program_of(&program.value)?;
    // The script or pattern of `-e` and its long forms is the next word, not a file.
    let operands: Vec<&ShellWord> = args
        .iter()
        .enumerate()
        .filter(|(at, word)| {
            let before = at.checked_sub(1).and_then(|before| args.get(before));
            !word.value.starts_with('-')
                && !before.is_some_and(|before| PATTERN_OPTIONS.contains(&before.value.as_str()))
        })
        .map(|(_, word)| word)
        .collect();
    let pattern_first = PATTERN_FIRST.contains(&program.value.as_str())
        && !args.iter().any(|word| gives_pattern(&word.value));
    let files = if pattern_first {
        operands.into_iter().skip(1).collect()
    } else {
        operands
    };
    let (reads, writes) = role_of(role, files, args);
    Some(Operands {
        reads,
        writes,
        checked,
    })
}

/// Every file a Bash command names or redirects to, in the order it runs: redirections and `tee`
/// (`redirectTargets`), then the operands of the file commands, read against the starting
/// directory.
#[must_use]
pub fn command_targets(command: &str) -> Vec<Target> {
    let mut targets = redirect_targets(command);
    let safe_env = HashSet::new();
    for subcommand in &scan_command(command, None).commands {
        let words = stripped_words(subcommand, PermissionBehavior::Deny, &safe_env);
        let Some(operands) = file_operands(&words) else {
            continue;
        };
        let source = if operands.checked {
            TargetSource::Operand
        } else {
            TargetSource::Unchecked
        };
        let of = |kind| move |word: &&ShellWord| Target::new(word, kind, false, source);
        targets.extend(operands.reads.iter().map(of(TargetKind::Input)));
        targets.extend(operands.writes.iter().map(of(TargetKind::Output)));
    }
    targets
}

/// The files a Bash command reads and writes by naming them (`BashFileTargets`).
#[must_use]
pub fn bash_file_targets(command: &str, paths: &PathContext) -> BashFileTargets {
    let mut found = BashFileTargets::default();
    for target in command_targets(command) {
        let Some(literal) = literal_of(&target.word, paths) else {
            continue;
        };
        match target.kind {
            TargetKind::Input => found.reads.push(literal),
            TargetKind::Output => found.writes.push(literal),
        }
    }
    found
}

crate::registry::exports! {
    "rules/decide/file-targets" {
        "bashFileTargets" => ok |command: String, paths: PathContext| bash_file_targets(&command, &paths);
    }
}
