//! Port of `src/rules/bash/wrappers.ts`, switched.
//!
//! The wrappers permissions.md ("Bash > Wrappers") says Claude Code strips before matching:
//! `timeout`, `time`, `nice`, `nohup` and `stdbuf`, the shell builtins `command` and `builtin`, zsh's
//! `noglob`, and bare `xargs`. "This wrapper list is built in and is not configurable."

use std::collections::HashSet;

use serde::Serialize;

use super::safe_env::{assigned_variable, is_safe_assignment};
use super::words::{ShellCommand, ShellWord, read_command};
use crate::js::slice_code_units;
use crate::registry::Set;
use crate::rules::restricts::restricts;
use crate::rules::types::PermissionBehavior;

/// What stands between a wrapper and the command it runs. The docs give the list, not the options,
/// so each wrapper's own are the kit's reading of its manual (spec/judgments.md).
struct WrapperSyntax {
    /// Options whose value is the next word (`timeout -s KILL`, `nice -n 5`).
    value_options: &'static [&'static str],
    /// Words after the options that are not the command (`timeout`'s duration).
    operands: usize,
    /// An option the wrapper may carry; `false` for `xargs`, which is stripped only when bare.
    takes_options: bool,
    /// Options that turn the wrapper into something else: `command -v` looks a command up.
    querying: &'static [&'static str],
}

const NONE: &[&str] = &[];

/// The wrappers, by name, in the order the docs list them.
const WRAPPERS: [(&str, WrapperSyntax); 9] = [
    (
        "timeout",
        WrapperSyntax {
            value_options: &["-s", "--signal", "-k", "--kill-after"],
            operands: 1,
            takes_options: true,
            querying: NONE,
        },
    ),
    (
        "time",
        WrapperSyntax {
            value_options: &["-f", "--format", "-o", "--output"],
            operands: 0,
            takes_options: true,
            querying: NONE,
        },
    ),
    (
        "nice",
        WrapperSyntax {
            value_options: &["-n", "--adjustment"],
            operands: 0,
            takes_options: true,
            querying: NONE,
        },
    ),
    (
        "nohup",
        WrapperSyntax {
            value_options: NONE,
            operands: 0,
            takes_options: true,
            querying: NONE,
        },
    ),
    (
        "stdbuf",
        WrapperSyntax {
            value_options: &["-i", "-o", "-e", "--input", "--output", "--error"],
            operands: 0,
            takes_options: true,
            querying: NONE,
        },
    ),
    (
        "command",
        WrapperSyntax {
            value_options: NONE,
            operands: 0,
            takes_options: true,
            querying: &["-v", "-V"],
        },
    ),
    (
        "builtin",
        WrapperSyntax {
            value_options: NONE,
            operands: 0,
            takes_options: false,
            querying: NONE,
        },
    ),
    (
        "noglob",
        WrapperSyntax {
            value_options: NONE,
            operands: 0,
            takes_options: false,
            querying: NONE,
        },
    ),
    (
        "xargs",
        WrapperSyntax {
            value_options: NONE,
            operands: 0,
            takes_options: false,
            querying: NONE,
        },
    ),
];

/// The wrapper a name is, if any (`isWrapper`).
fn wrapper(name: &str) -> Option<&'static WrapperSyntax> {
    WRAPPERS
        .iter()
        .find(|(wrapper_name, _)| *wrapper_name == name)
        .map(|(_, syntax)| syntax)
}

/// Index of the command a wrapper at `words[from]` runs, or `None` when it runs none.
fn command_index(words: &[ShellWord], from: usize, syntax: &WrapperSyntax) -> Option<usize> {
    let mut index = from + 1;
    while words
        .get(index)
        .is_some_and(|word| word.value.starts_with('-'))
    {
        let option = words.get(index).map_or("", |word| word.value.as_str());
        if option == "--" {
            index += 1;
            break;
        }
        if !syntax.takes_options || syntax.querying.contains(&option) {
            return None;
        }
        index += if syntax.value_options.contains(&option) {
            2
        } else {
            1
        };
    }
    index += syntax.operands;
    (index < words.len()).then_some(index)
}

/// The index after one leading assignment or wrapper at `from`, or `None` when there is
/// neither. "An allow rule won't match past an assignment of any other variable. A deny or ask rule
/// matches past any leading assignment" (`RESTRICTS`).
fn strip_step(
    words: &[ShellWord],
    from: usize,
    behavior: PermissionBehavior,
    safe_env: &HashSet<String>,
) -> Option<usize> {
    let first = words.get(from)?;
    if assigned_variable(first).is_some() {
        let strips = restricts(behavior) || is_safe_assignment(first, safe_env);
        return (strips && words.get(from + 1).is_some()).then_some(from + 1);
    }
    // A quoted wrapper name runs the wrapper all the same
    // (crates/agent-plugins/src/rules/bash/unquote.rs).
    wrapper(&first.value).and_then(|syntax| command_index(words, from, syntax))
}

/// Where each form of a command starts among its words: at its first word, then after each wrapper
/// or assignment is removed in turn, so `timeout 5 nohup npm test` is also `nohup npm test` and `npm
/// test`.
#[must_use]
pub fn form_starts(
    words: &[ShellWord],
    behavior: PermissionBehavior,
    safe_env: &HashSet<String>,
) -> Vec<usize> {
    let mut starts = vec![0];
    let mut next = strip_step(words, 0, behavior, safe_env);
    while let Some(start) = next {
        starts.push(start);
        next = strip_step(words, start, behavior, safe_env);
    }
    starts
}

/// The words of the command a subcommand runs once every leading wrapper and assignment is removed.
#[must_use]
pub fn stripped_words(
    command: &ShellCommand,
    behavior: PermissionBehavior,
    safe_env: &HashSet<String>,
) -> Vec<ShellWord> {
    let start = form_starts(&command.words, behavior, safe_env)
        .last()
        .copied()
        .unwrap_or(0);
    command.words.get(start..).unwrap_or_default().to_vec()
}

/// The command a subcommand runs once every leading wrapper and assignment is removed, as written.
#[must_use]
pub fn strip_wrappers(
    subcommand: &str,
    behavior: PermissionBehavior,
    safe_env: &HashSet<String>,
) -> String {
    let command = read_command(subcommand, None);
    let start = form_starts(&command.words, behavior, safe_env)
        .last()
        .copied()
        .unwrap_or(0);
    // `first === command.words[0]`: the first word of the slice is the command's own first word.
    match command.words.get(start) {
        Some(first) if start > 0 => slice_code_units(subcommand, first.start, None),
        Some(_) | None => subcommand.to_owned(),
    }
}

type ReadOnlyStage = fn(&[ShellWord], usize) -> Option<usize>;

/// What the read-only set sees through, in the order Claude Code accepts them (oracle 2.1.322):
/// known-safe assignments, then `noglob`, then `command` (`-p`, `--`) or `builtin`, then `timeout`
/// with a bare duration. A later stage never comes before an earlier one (`timeout 5 command ls`,
/// `command LANG=C ls` and `command noglob ls` were asked), and nothing else is stripped: `nice`,
/// `time`, `nohup`, `stdbuf`, `xargs` and any `timeout` option were asked (spec/judgments.md).
const READ_ONLY_STAGES: [ReadOnlyStage; 4] = [
    |words, at| {
        let word = words.get(at)?;
        is_safe_assignment(word, &HashSet::new()).then_some(at + 1)
    },
    |words, at| (words.get(at)?.value == "noglob").then_some(at + 1),
    |words, at| {
        let name = words.get(at)?.value.as_str();
        if name == "builtin" {
            return Some(at + 1);
        }
        if name != "command" {
            return None;
        }
        let mut next = at + 1;
        while words
            .get(next)
            .is_some_and(|word| word.value == "-p" || word.value == "--")
        {
            next += 1;
        }
        if words
            .get(next)
            .is_some_and(|word| word.value.starts_with('-'))
        {
            None
        } else {
            Some(next)
        }
    },
    |words, at| {
        let is_timeout = words.get(at)?.value == "timeout";
        let duration = words.get(at + 1).map_or("", |word| word.value.as_str());
        (is_timeout && is_duration(duration)).then_some(at + 2)
    },
];

/// `DURATION`, `/^\d+(?:\.\d+)?[smhd]?$/`.
fn is_duration(text: &str) -> bool {
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let number = text.strip_suffix(['s', 'm', 'h', 'd']).unwrap_or(text);
    match number.split_once('.') {
        None => digits(number),
        Some((whole, fraction)) => digits(whole) && digits(fraction),
    }
}

/// What `readOnlyWords` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReadOnlyWords {
    /// The words of the command the read-only set judges.
    pub words: Vec<ShellWord>,
    /// Whether any wrapper was removed.
    pub stripped: bool,
}

/// The words of the command the read-only set judges once the wrappers it sees through are removed
/// (`READ_ONLY_STAGES`), and whether any was: `git` and `cd` behind one are not read-only.
#[must_use]
pub fn read_only_words(words: &[ShellWord]) -> ReadOnlyWords {
    let mut at = 0;
    let mut stage = 0;
    while stage < READ_ONLY_STAGES.len() {
        let next = READ_ONLY_STAGES
            .get(stage)
            .and_then(|stage_of| stage_of(words, at));
        match next {
            Some(next) if next < words.len() => at = next,
            Some(_) | None => stage += 1,
        }
    }
    ReadOnlyWords {
        words: words.get(at..).unwrap_or_default().to_vec(),
        stripped: at > 0,
    }
}

/// `safeEnv` as the host's set, or the empty one.
fn safe_env_of(safe_env: Option<Set<String>>) -> HashSet<String> {
    safe_env
        .map(|set| set.members.into_iter().collect())
        .unwrap_or_default()
}

crate::registry::exports! {
    "rules/bash/wrappers" {
        "strippedWords" => ok |command: ShellCommand, behavior: PermissionBehavior, safe_env: Option<Set<String>>| stripped_words(&command, behavior, &safe_env_of(safe_env));
        "stripWrappers" => ok |subcommand: String, behavior: PermissionBehavior, safe_env: Option<Set<String>>| strip_wrappers(&subcommand, behavior, &safe_env_of(safe_env));
        "readOnlyWords" => ok |words: Vec<ShellWord>| read_only_words(&words);
    }
}
