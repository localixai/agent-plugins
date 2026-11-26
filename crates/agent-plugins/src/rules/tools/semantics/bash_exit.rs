//! Port of `src/rules/tools/semantics/bash-exit.ts`, switched.

use crate::js::{is_whitespace, trim_end};
use crate::rules::bash::split::split_command;
use crate::rules::bash::words::words_of;

/// tools-reference.md, "Output limits": "A command that exits 1 counts as a valid result for the
/// Bash tool only when Claude Code recognizes exit code 1 as a benign outcome for that command:
/// `grep`, `rg`, `egrep`, `fgrep`, `find`, `diff`, `test`, and `[`, plus `git diff` and `git grep`."
/// Each entry is the command's leading words.
const BENIGN_EXIT_1: [&[&str]; 10] = [
    &["grep"],
    &["rg"],
    &["egrep"],
    &["fgrep"],
    &["find"],
    &["diff"],
    &["test"],
    &["["],
    &["git", "diff"],
    &["git", "grep"],
];

/// tools-reference.md, "Exit code 1 from searches and `git diff` in PowerShell": "The PowerShell
/// tool can count exit code 1 as a normal result from `grep`, `rg`, `egrep`, `fgrep`, `findstr`,
/// `git grep`, and `git diff`."
const BENIGN_EXIT_1_POWERSHELL: [&[&str]; 7] = [
    &["grep"],
    &["rg"],
    &["egrep"],
    &["fgrep"],
    &["findstr"],
    &["git", "grep"],
    &["git", "diff"],
];

/// The simple command a line exits with: the last one, found as the subcommand that ends the text.
/// A `!` before it inverts the status, so the line is not read as that command (spec/judgments.md).
fn last_command(command: &str) -> Option<String> {
    let text = trim_end(command)
        .trim_end_matches(|character| character == ';' || is_whitespace(character));
    let subcommands = split_command(text, None).subcommands;
    let found = subcommands
        .iter()
        .rfind(|subcommand| text.ends_with(subcommand.as_str()))?;
    let before = trim_end(text.get(..text.len() - found.len())?);
    if before.ends_with('!') {
        None
    } else {
        Some(found.clone())
    }
}

fn begins_with(words: &[String], head: &[&str]) -> bool {
    head.iter()
        .enumerate()
        .all(|(index, word)| words.get(index).is_some_and(|candidate| candidate == word))
}

fn classify(benign: &[&[&str]], command: &str, exit_code: f64) -> bool {
    if exit_code == 0.0 {
        return true;
    }
    if exit_code != 1.0 {
        return false;
    }
    let Some(last) = last_command(command) else {
        return false;
    };
    let words: Vec<String> = words_of(&last, None)
        .into_iter()
        .map(|word| word.text)
        .collect();
    benign.iter().any(|head| begins_with(&words, head))
}

/// Whether Claude Code counts the finished command as a valid result rather than a failure: exit 0
/// always, exit 1 only for the commands above, anything else never. A compound line exits with its
/// last command, and a command behind a wrapper, a variable assignment, a `!`, a subshell or a
/// trailing `&`, or with options before the subcommand (`sudo grep`, `git -C . diff`), is read as
/// written, so it is a failure (spec/judgments.md).
#[must_use]
pub fn classify_bash_exit(command: &str, exit_code: f64) -> bool {
    classify(&BENIGN_EXIT_1, command, exit_code)
}

/// The same for the PowerShell tool, whose benign list is its own (`findstr` in, `find`, `diff`,
/// `test` and `[` out). A line is split as a Bash line is (spec/judgments.md).
#[must_use]
pub fn classify_power_shell_exit(command: &str, exit_code: f64) -> bool {
    classify(&BENIGN_EXIT_1_POWERSHELL, command, exit_code)
}

crate::registry::exports! {
    "rules/tools/semantics/bash-exit" {
        "classifyBashExit" => ok |command: String, exit_code: f64| classify_bash_exit(&command, exit_code);
        "classifyPowerShellExit" => ok |command: String, exit_code: f64| classify_power_shell_exit(&command, exit_code);
    }
}
