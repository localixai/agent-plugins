//! Port of `src/skills/shell-run.ts`, switched.
//!
//! Running the commands a skill injects (skills.md, "Inject dynamic context", "How injected commands
//! run", "When an injected command fails", "Permission checks on injected commands"; errors.md, "No
//! shell command in a skill or command was run"). Every effect is the `CommandRunner` port; the
//! permission answer is the host's (`ShellPermission`), since the deny, ask and allow engine and the
//! grants of `allowed-tools` meet the session's rules there.
//!
//! The ports cross as a value the host keeps (`permissions` is a function), so each is read as its
//! member; the runner is a `SkillCommandRunner` (`src/skills/shell-ports.ts`), whose run and result
//! carry the move to the background, so it is asked by its method `run`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::shell_refusal::refusal_of;
use super::shell_result::{
    Planned, ResultContext, SkillCommandResult, Step, may_move_to_background, render_result,
    timeout_of,
};
use crate::agents::permission_baseline::PermissionMode;
use crate::format::issues::{PluginIssue, error};
use crate::js::in_key_order;
use crate::js::value::to_js_value;
use crate::ports::Signal;
use crate::ports::command_runner::{CommandSpec, ShellKind};
use crate::ports::scratch_files::ScratchFiles;
use crate::registry::{Callback, Port};
use crate::rules::types::PermissionBehavior;

/// The tool a command runs through, as the permission rules name it.
const fn tool_name(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Bash => "Bash",
        ShellKind::Powershell => "PowerShell",
    }
}

/// What the host's permission answer is asked about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShellCall {
    /// `Bash` or `PowerShell`.
    pub tool: &'static str,
    /// The command after substitution with its paths written in, as data: the text a permission
    /// rule is written against (skills.md: an `allowed-tools` rule "matches the exact command").
    /// Under the default `shellPaths` the shell runs the same command with each path a reference
    /// to its environment, so a quote or `$(...)` in a folder name is never parsed
    /// (spec/judgments.md).
    pub command: String,
    /// The skill's `allowed-tools` with `${CLAUDE_*}` substituted. "Deny and ask rules still
    /// override `allowed-tools`": the host builds its answer from its rules, then these grants,
    /// then the mode.
    pub grants: Vec<String>,
}

/// One command to run: what is checked, what the shell gets, and its placeholder (for messages).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedCommand {
    /// The placeholder.
    pub pattern: String,
    /// The command with its paths written in: what the refusal, the rules and the messages read.
    pub command: String,
    /// What the shell runs: `command` with its paths as the host's `shellPaths` writes them.
    pub shell_command: String,
    /// The values of the references `shellCommand` holds, added to the command's environment.
    pub environment: IndexMap<String, String>,
}

/// What the commands of one invocation share.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InjectionContext {
    /// The command name with its slash, as `No shell command in <name> was run` quotes it.
    pub name: String,
    /// The shell.
    pub shell: ShellKind,
    /// The working directory.
    pub cwd: String,
    /// The environment.
    pub env: IndexMap<String, String>,
    /// The skill's grants.
    pub grants: Vec<String>,
    /// The session's permission mode.
    #[serde(default)]
    pub mode: Option<PermissionMode>,
    /// A forked skill that sets `agent`: the permission check always aborts there.
    pub forked_with_agent: bool,
    /// Ends the runs.
    pub signal: Signal,
    /// The `bashOutputMaxChars` setting: how much of a result arrives inline.
    #[serde(default)]
    pub bash_output_max_chars: Option<f64>,
    /// Bare mode: a command that reaches its timeout stops instead of moving to the background.
    #[serde(default)]
    pub bare: Option<bool>,
}

/// The outputs in order, or the issue of the first failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum InjectionResults {
    /// Every command ran.
    Ran {
        /// Always true.
        ok: bool,
        /// Each command's replacement.
        outputs: Vec<String>,
    },
    /// One failed.
    Failed {
        /// Always false.
        ok: bool,
        /// Why.
        issue: PluginIssue,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Run,
    Abort,
    Defer,
}

/// Only auto mode lets a command that needs approval through with an instruction.
fn defers_ask(mode: Option<PermissionMode>) -> bool {
    mode == Some(PermissionMode::Auto)
}

/// What each answer means (skills.md, "Permission checks on injected commands"). An answer that
/// is not allow aborts, except in auto mode, where an ask "doesn't abort the invocation": the skill
/// loads with an instruction to run the command first. That does not hold "in a forked skill that
/// sets `agent`".
fn verdict_of(behavior: PermissionBehavior, context: &InjectionContext) -> Verdict {
    match behavior {
        PermissionBehavior::Allow => Verdict::Run,
        PermissionBehavior::Deny => Verdict::Abort,
        PermissionBehavior::Ask if defers_ask(context.mode) && !context.forked_with_agent => {
            Verdict::Defer
        }
        PermissionBehavior::Ask => Verdict::Abort,
    }
}

/// Judgment (spec/judgments.md): the instruction that takes the place of a command auto mode did
/// not run; the docs give its intent ("an instruction telling Claude to run the command first").
fn instruction_for(command: &str) -> String {
    format!(
        "[This command needs approval and was not run. Run it first, then use its output here: {command}]"
    )
}

/// The context the result rules read.
fn result_context(context: &InjectionContext) -> ResultContext {
    ResultContext {
        shell: context.shell,
        env: context.env.clone(),
        cwd: context.cwd.clone(),
        bash_output_max_chars: context.bash_output_max_chars,
        bare: context.bare == Some(true),
    }
}

/// The ports, read from the host's record.
struct InjectionPorts {
    runner: Port,
    permissions: Callback,
    scratch: Option<Port>,
}

impl InjectionPorts {
    async fn of(ports: Port) -> Self {
        Self {
            runner: ports.member("runner").await,
            permissions: ports.member("permissions").await,
            scratch: ports.member("scratch").await,
        }
    }
}

/// A `SkillCommandRun`: a `CommandRun` that may move to the background at its timeout.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillCommandRun<'a> {
    spec: CommandSpec,
    cwd: &'a str,
    env: IndexMap<String, String>,
    timeout_ms: f64,
    merge_stderr: bool,
    move_to_background_on_timeout: bool,
    signal: &'a Signal,
}

async fn execute(
    planned: &PlannedCommand,
    context: &InjectionContext,
    ports: &InjectionPorts,
) -> Step {
    let shell = context.shell;
    let rules = result_context(context);
    let mut env = context.env.clone();
    env.extend(planned.environment.clone());
    let run = SkillCommandRun {
        spec: CommandSpec::Shell {
            command: planned.shell_command.clone(),
            shell,
        },
        cwd: &context.cwd,
        env: in_key_order(env),
        timeout_ms: timeout_of(&context.env).await,
        // "With the default `bash` shell, Claude Code merges stderr into stdout."
        merge_stderr: shell == ShellKind::Bash,
        move_to_background_on_timeout: may_move_to_background(&planned.command, &rules).await,
        signal: &context.signal,
    };
    let run = to_js_value(&run).unwrap_or_default();
    let result: SkillCommandResult = ports.runner.method("run", vec![run]).await;
    let scratch = ports
        .scratch
        .as_ref()
        .map(|scratch| scratch as &dyn ScratchFiles);
    let planned = Planned {
        pattern: planned.pattern.clone(),
        command: planned.command.clone(),
    };
    render_result(&planned, &result, &rules, (scratch, &context.signal)).await
}

/// One command, with a file's CRLF line endings read as plain newlines: the refusal check and the
/// permission rules see the same text, and the shell that text with its paths as references.
async fn step(given: &PlannedCommand, context: &InjectionContext, ports: &InjectionPorts) -> Step {
    let planned = PlannedCommand {
        command: given.command.replace("\r\n", "\n"),
        shell_command: given.shell_command.replace("\r\n", "\n"),
        ..given.clone()
    };
    if let Some(refused) = refusal_of(&planned.command, &context.name, context.shell) {
        return Step::Failure { failure: refused };
    }
    let call = ShellCall {
        tool: tool_name(context.shell),
        command: planned.command.clone(),
        grants: context.grants.clone(),
    };
    let call = to_js_value(&call).unwrap_or_default();
    let answer: PermissionBehavior = ports.permissions.call(vec![call]).await;
    match verdict_of(answer, context) {
        Verdict::Abort => Step::Failure {
            failure: format!(
                "Shell command permission check failed for pattern \"{}\"",
                planned.pattern
            ),
        },
        Verdict::Defer => Step::Output {
            output: instruction_for(&planned.command),
        },
        Verdict::Run => execute(&planned, context, ports).await,
    }
}

/// Runs the commands one after another, in order (they share the working directory). The first
/// failure aborts the whole invocation: "Claude never sees the skill content for that invocation".
pub async fn run_injections(
    commands: &[PlannedCommand],
    context: &InjectionContext,
    ports: Port,
) -> InjectionResults {
    let ports = InjectionPorts::of(ports).await;
    let mut outputs = Vec::new();
    for planned in commands {
        match step(planned, context, &ports).await {
            Step::Failure { failure } => {
                return InjectionResults::Failed {
                    ok: false,
                    issue: error(&context.name, &failure),
                };
            }
            Step::Output { output } => outputs.push(output),
        }
    }
    InjectionResults::Ran { ok: true, outputs }
}

crate::registry::exports! {
    "skills/shell-run" {
        "runInjections" => async ok |commands: Vec<PlannedCommand>, context: InjectionContext, ports: Port|
            run_injections(&commands, &context, ports).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ask_defers_only_in_auto_mode_outside_a_forked_agent() {
        let context = |mode, forked_with_agent| InjectionContext {
            name: "/x".to_owned(),
            shell: ShellKind::Bash,
            cwd: "/".to_owned(),
            env: IndexMap::new(),
            grants: Vec::new(),
            mode,
            forked_with_agent,
            signal: Signal::new(),
            bash_output_max_chars: None,
            bare: None,
        };
        let ask = PermissionBehavior::Ask;
        assert!(verdict_of(ask, &context(Some(PermissionMode::Auto), false)) == Verdict::Defer);
        assert!(verdict_of(ask, &context(Some(PermissionMode::Auto), true)) == Verdict::Abort);
        assert!(verdict_of(ask, &context(None, false)) == Verdict::Abort);
    }
}
