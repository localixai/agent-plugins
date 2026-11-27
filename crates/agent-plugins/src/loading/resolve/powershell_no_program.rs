//! Port of `src/loading/resolve/powershell-no-program.ts`, switched.
//!
//! hooks.md, "PowerShell started no program": "A hook with this error ran in PowerShell, which prints a
//! `command` that is only quoted strings, such as `"command": "\"C:\\Tools\\guard.exe\""`, instead of
//! running it." The fixes: `"shell": "powershell"` "to print text, such as JSON, and the hook exits 0";
//! `"shell": "powershell"` and `&` before the quoted path to run a script or a `.exe` whose path has a
//! variable; no inner quotes for one `.exe` at a path without `$`; exec form (`command` and `args`) or
//! Git Bash for a script written for Bash.
//!
//! Judgment (spec/judgments.md): the docs give the error's name and fixes, not when the kit may say it.
//! It says it for a shell-form hook that runs in PowerShell by the host's default (no `shell` field)
//! whose command is only quoted strings; "set `shell`: `powershell`" is the fix that makes the hook
//! exit 0, so a hook that names the shell is the author's choice and runs as written. A string is
//! `"..."` (a backtick or `""` inside is an escape) or `'...'` (`''` inside is one quote); several
//! strings separated by white space are still only strings.

use std::sync::LazyLock;

use regex::Regex;

use crate::format::issues::{PluginIssue, error};
use crate::js::{Pattern, SPACE_CLASS, regex_test};
use crate::ports::command_runner::ShellKind;

const DOUBLE: &str = "\"(?:[^\"`]|`(?s:.)|\"\")*\"";
const SINGLE: &str = "'(?:[^']|'')*'";

static ONLY_QUOTED_STRINGS: Pattern = LazyLock::new(|| {
    let space = format!("[{SPACE_CLASS}]");
    Regex::new(&format!(
        "^{space}*(?:{DOUBLE}|{SINGLE})(?:{space}+(?:{DOUBLE}|{SINGLE}))*{space}*$"
    ))
    .ok()
});

/// Whether `command` is only quoted strings.
#[must_use]
pub fn only_quoted_strings(command: &str) -> bool {
    regex_test(&ONLY_QUOTED_STRINGS, command)
}

/// What the hook's error says: the docs' sentence first, then the fixes.
pub const NO_PROGRAM: &str = concat!(
    "PowerShell started no program: the command is only quoted strings, which PowerShell prints instead of running. ",
    "Set \"shell\": \"powershell\" to print the text, put & before the quoted path to run a script, ",
    "remove the inner quotes to run one .exe, or use exec form (\"command\" and \"args\"), or install Git Bash for a script written for Bash."
);

/// The hook does not start: it runs in PowerShell (`shell`) because the host defaults to it
/// (`named` is the handler's own `shell` field, absent) and the command is only quoted strings.
#[must_use]
pub fn no_program_issue(
    shell: ShellKind,
    named: Option<ShellKind>,
    path: &str,
    command: &str,
) -> Option<PluginIssue> {
    (shell == ShellKind::Powershell && named.is_none() && only_quoted_strings(command))
        .then(|| error(path, NO_PROGRAM))
}

crate::registry::exports! {
    "loading/resolve/powershell-no-program" {
        "onlyQuotedStrings" => ok |command: String| only_quoted_strings(&command);
        "NO_PROGRAM" => ok || NO_PROGRAM;
        "noProgramIssue" => ok |shell: ShellKind, named: Option<ShellKind>, path: String, command: String| no_program_issue(shell, named, &path, &command);
    }
}
