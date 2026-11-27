//! Port of `src/rules/bash/shell.ts`, switched.
//!
//! What differs between the command tools, the one matcher below reading it: how a command
//! splits, the forms of a subcommand a rule is tried against, and how a pattern is compared with
//! one. `matchShellRule` is not exported: its callers, the Bash and PowerShell matchers, are ported
//! too, so only the core reads a `Shell`.

use std::collections::HashSet;

use super::dialect::ShellDialect;
use super::exec_wrappers::prompts_despite_prefix_rule;
use super::split::scan_command;
use super::text_pattern::text_pattern_of;
use super::wildcard::{has_wildcard, normalize_pattern};
use super::words::ShellCommand;
use crate::js::JsValue;
use crate::rules::restricts::restricts;
use crate::rules::types::{MatchContext, PermissionBehavior, PermissionRule, PreparedCall};

/// The forms of one subcommand a rule is tried against (wrappers stripped, aliases resolved).
pub type Forms = fn(&ShellCommand, PermissionBehavior, &HashSet<String>) -> Vec<String>;

/// One command tool's way with commands (`Shell`).
pub struct Shell {
    /// How a command splits.
    pub dialect: &'static ShellDialect,
    /// The forms of one subcommand a rule is tried against.
    pub forms: Forms,
    /// The pattern as it is compared: aliases resolved for PowerShell.
    pub pattern: fn(&str) -> String,
    /// Whether a pattern matches a form.
    pub compare: fn(&str, &str) -> bool,
    /// Prefix allow rules cannot approve exec wrappers and find actions (a Bash rule).
    pub guards_exec_wrappers: bool,
}

/// The field that holds the command: the SDK's `BashInput` and `MonitorInput` both have it.
const COMMAND_FIELD: &str = "command";

/// The command text a call carries: `command` for Bash, Monitor and PowerShell (hooks.md). The
/// input is `unknown` because a hook or a model sends whatever it likes; the field name is the
/// SDK's.
#[must_use]
pub fn command_of(input: &JsValue) -> Option<&str> {
    input.get(COMMAND_FIELD).and_then(JsValue::as_str)
}

/// How many of a command's subcommands a rule must cover (permissions.md, "Compound commands"):
/// every one for an allow rule, which never matches a command it could not read to the end; any
/// one for a deny or ask rule, which also tries the whole text of a command it could not read.
fn coverage(
    behavior: PermissionBehavior,
    commands: &[ShellCommand],
    parseable: bool,
    covered: impl Fn(&ShellCommand) -> bool,
    whole: &ShellCommand,
) -> bool {
    match behavior {
        PermissionBehavior::Allow => {
            parseable && !commands.is_empty() && commands.iter().all(covered)
        }
        PermissionBehavior::Deny | PermissionBehavior::Ask => {
            commands.iter().any(&covered) || (!parseable && covered(whole))
        }
    }
}

/// The variables an allow rule matches past beyond the known-safe ones (`safe-env.ts`): the
/// host's own.
fn safe_env_of(ctx: &MatchContext) -> HashSet<String> {
    ctx.safe_env_vars
        .as_ref()
        .map(|set| set.members.iter().cloned().collect())
        .unwrap_or_default()
}

/// Whether a text-specifier rule on a command tool covers the call's command. A prefix (wildcard)
/// allow rule, a granting rule, must not approve exec wrappers and find actions (a Bash rule).
#[must_use]
pub fn match_shell_rule(
    shell: &Shell,
    rule: &PermissionRule,
    call: &PreparedCall,
    ctx: &MatchContext,
) -> bool {
    let behavior = rule.behavior;
    let (Some(text), Some(command)) = (text_pattern_of(rule), command_of(&call.call.input)) else {
        return false;
    };
    let pattern = (shell.pattern)(&normalize_pattern(text));
    let safe_env = safe_env_of(ctx);
    let prefix_rule = !restricts(behavior) && shell.guards_exec_wrappers && has_wildcard(&pattern);
    let covered = |subcommand: &ShellCommand| {
        !(prefix_rule && prompts_despite_prefix_rule(subcommand))
            && (shell.forms)(subcommand, behavior, &safe_env)
                .iter()
                .any(|form| (shell.compare)(&pattern, form))
    };
    let scan = scan_command(command, Some(shell.dialect));
    coverage(
        behavior,
        &scan.commands,
        scan.parseable,
        covered,
        &scan.whole,
    )
}

crate::registry::exports! {
    "rules/bash/shell" {
        "commandOf" => ok |input: JsValue| command_of(&input).map(str::to_owned);
    }
}
