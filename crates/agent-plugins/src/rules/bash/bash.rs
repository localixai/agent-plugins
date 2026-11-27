//! Port of `src/rules/bash/bash.ts`, switched.
//!
//! `Bash(...)` and `Monitor(...)` rules with a command pattern (tools-reference.md, "Rule
//! format"): wrappers stripped, patterns compared as written.

use std::collections::HashSet;

use indexmap::IndexSet;

use super::dialect::BASH_DIALECT;
use super::shell::{Shell, match_shell_rule};
use super::wildcard::{PatternOptions, match_bash_pattern};
use super::words::{ShellCommand, spoken_form};
use super::wrappers::form_starts;
use crate::js::slice_code_units;
use crate::rules::restricts::restricts;
use crate::rules::types::{MatchContext, PermissionBehavior, PermissionRule, PreparedCall};

/// The form that starts at word `start`: as written, and with its command name unquoted.
fn written_forms(command: &ShellCommand, start: usize) -> Vec<String> {
    let text = &command.text;
    let Some(name) = command.words.get(start) else {
        return vec![text.clone()];
    };
    let from = if start == 0 { 0 } else { name.start };
    vec![
        slice_code_units(text, from, None),
        format!(
            "{}{}{}",
            slice_code_units(text, from, Some(name.start)),
            name.value,
            slice_code_units(text, name.end, None)
        ),
    ]
}

/// Each form a subcommand takes once wrappers are stripped, as written and with its command name
/// unquoted (crates/agent-plugins/src/rules/bash/unquote.rs): `"rm" x` is `rm x` to a rule. A
/// deny or ask rule also reads the words as the shell passes them, one space apart and without
/// the redirections, so a tab, a line continuation or `>out` in front of the name does not hide
/// the command from it.
fn bash_forms(
    command: &ShellCommand,
    behavior: PermissionBehavior,
    safe_env: &HashSet<String>,
) -> Vec<String> {
    let starts = form_starts(&command.words, behavior, safe_env);
    let spoken: Vec<String> = if restricts(behavior) {
        starts
            .iter()
            .map(|start| {
                spoken_form(
                    command.words.get(*start..).unwrap_or_default(),
                    &BASH_DIALECT,
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    let forms: IndexSet<String> = starts
        .iter()
        .flat_map(|start| written_forms(command, *start))
        .chain(spoken)
        .collect();
    forms.into_iter().collect()
}

/// The Bash and Monitor command format.
const BASH: Shell = Shell {
    dialect: &BASH_DIALECT,
    forms: bash_forms,
    pattern: str::to_owned,
    compare: |pattern, form| match_bash_pattern(pattern, form, PatternOptions::default()),
    guards_exec_wrappers: true,
};

/// `Bash(...)` and `Monitor(...)` rules with a command pattern (tools-reference.md, "Rule
/// format").
#[must_use]
pub fn match_bash_rule(rule: &PermissionRule, call: &PreparedCall, ctx: &MatchContext) -> bool {
    match_shell_rule(&BASH, rule, call, ctx)
}

crate::registry::exports! {
    "rules/bash/bash" {
        "matchBashRule" => ok |rule: PermissionRule, call: PreparedCall, ctx: MatchContext| match_bash_rule(&rule, &call, &ctx);
    }
}
