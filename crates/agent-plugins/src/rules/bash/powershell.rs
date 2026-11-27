//! Port of `src/rules/bash/powershell.ts`, switched.
//!
//! `PowerShell(...)` rules with a command pattern: no wrappers, aliases resolved on both sides,
//! case ignored.

use std::collections::HashSet;

use serde::Serialize;

use super::dialect::POWERSHELL_DIALECT;
use super::powershell_aliases::POWERSHELL_ALIASES;
use super::shell::{Shell, match_shell_rule};
use super::wildcard::{PatternOptions, match_bash_pattern};
use super::words::{ShellCommand, ShellWord, read_command};
use crate::js::slice_code_units;
use crate::rules::restricts::restricts;
use crate::rules::types::{MatchContext, PermissionBehavior, PermissionRule, PreparedCall};

/// `&` and `.` run the command named after them: `& Remove-Item x` runs `Remove-Item`.
const CALL_OPERATORS: [&str; 2] = ["&", "."];

/// `Microsoft.PowerShell.Management\Remove-Item`: a module-qualified name is the cmdlet. The
/// length of `/^(?=[^\\]*[A-Za-z])[A-Za-z0-9_.]+\\(?=[^\\])/` at the start of `name`: the module
/// and its backslash, when the module has a letter and a name follows.
fn module_qualifier(name: &str) -> Option<usize> {
    let (module, rest) = name.split_once('\\')?;
    let word =
        |character: char| character.is_ascii_alphanumeric() || matches!(character, '_' | '.');
    let qualifies = !module.is_empty()
        && module.chars().all(word)
        && module
            .chars()
            .any(|character| character.is_ascii_alphabetic())
        && rest.chars().next().is_some_and(|next| next != '\\');
    qualifies.then_some(module.len() + 1)
}

/// The index of the word that names the command: the first, or the one after a call operator.
fn name_at(words: &[ShellWord]) -> usize {
    let first = words.first().map_or("", |word| word.value.as_str());
    usize::from(CALL_OPERATORS.contains(&first))
}

/// The cmdlet a name stands for: unquoted, its module dropped, an alias resolved.
fn cmdlet_of(name: &ShellWord) -> String {
    let bare = module_qualifier(&name.value)
        .and_then(|cut| name.value.get(cut..))
        .unwrap_or(&name.value);
    POWERSHELL_ALIASES
        .get(bare.to_lowercase().as_str())
        .map_or_else(|| bare.to_owned(), |cmdlet| (*cmdlet).to_owned())
}

/// The cmdlet a PowerShell command runs, and the words after its name.
#[derive(Clone, Debug, Serialize)]
pub struct CmdletCall {
    /// The cmdlet.
    pub cmdlet: String,
    /// The words after its name.
    pub args: Vec<ShellWord>,
}

/// The cmdlet a PowerShell command runs, and the words after its name; `None` with no name.
#[must_use]
pub fn cmdlet_call(command: &ShellCommand) -> Option<CmdletCall> {
    let at = name_at(&command.words);
    let name = command.words.get(at)?;
    Some(CmdletCall {
        cmdlet: cmdlet_of(name),
        args: command.words.get(at + 1..).unwrap_or_default().to_vec(),
    })
}

/// The command with its name as the cmdlet it runs, the rest as written.
fn written_form(command: &ShellCommand) -> String {
    match command.words.get(name_at(&command.words)) {
        None => command.text.clone(),
        Some(name) => format!(
            "{}{}",
            cmdlet_of(name),
            slice_code_units(&command.text, name.end, None)
        ),
    }
}

/// The command with its name as the cmdlet it runs and its words one space apart.
fn spoken_form(command: &ShellCommand) -> String {
    let at = name_at(&command.words);
    let Some(name) = command.words.get(at) else {
        return command.text.clone();
    };
    let rest = command.words.get(at + 1..).unwrap_or_default();
    std::iter::once(cmdlet_of(name))
        .chain(rest.iter().map(|word| word.text.clone()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The command (or pattern) with its first word, when it names a cmdlet another way, as the
/// cmdlet.
#[must_use]
pub fn canonicalize_power_shell(command: &str) -> String {
    written_form(&read_command(command, Some(&POWERSHELL_DIALECT)))
}

fn power_shell_forms(
    command: &ShellCommand,
    behavior: PermissionBehavior,
    _safe_env: &HashSet<String>,
) -> Vec<String> {
    if restricts(behavior) {
        vec![written_form(command), spoken_form(command)]
    } else {
        vec![written_form(command)]
    }
}

/// PowerShell: no wrappers, aliases resolved on both sides, case ignored.
const POWERSHELL: Shell = Shell {
    dialect: &POWERSHELL_DIALECT,
    forms: power_shell_forms,
    pattern: canonicalize_power_shell,
    compare: |pattern, form| {
        match_bash_pattern(
            pattern,
            form,
            PatternOptions {
                ignore_case: Some(true),
            },
        )
    },
    guards_exec_wrappers: false,
};

/// `PowerShell(...)` rules with a command pattern.
#[must_use]
pub fn match_power_shell_rule(
    rule: &PermissionRule,
    call: &PreparedCall,
    ctx: &MatchContext,
) -> bool {
    match_shell_rule(&POWERSHELL, rule, call, ctx)
}

crate::registry::exports! {
    "rules/bash/powershell" {
        "canonicalizePowerShell" => ok |command: String| canonicalize_power_shell(&command);
    }
}
