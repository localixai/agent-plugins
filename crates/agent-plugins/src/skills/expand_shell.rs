//! Ported from `src/skills/expand-shell.ts`; only the core uses it now, so it has no wrapper.
//!
//! What each `` !`command` `` placeholder of a skill becomes (skills.md, "Inject dynamic context"):
//! its command's output, the `disableSkillShellExecution` placeholder, or its own text.

mod request;

use indexmap::IndexMap;

pub use request::{ExpandPorts, ExpandRequest, ExpandSession, RequestGiven, Source};

use super::execution::resolve_shell;
use super::expand_policy::SkillOriginKind;
use super::shell_arguments::{bind_shell_arguments, binds_arguments};
use super::shell_run::{InjectionContext, InjectionResults, PlannedCommand, run_injections};
use super::shell_scan::Injection;
use super::shell_variables::substitute_shell_command;
use super::substitute::ArgumentInput;
use super::variables::{SkillVariables, substitute_variables, traits_of};
use crate::format::issues::{PluginIssue, error};
use crate::format::skill_types::SkillShell;
use crate::js::promise_all;
use crate::js::value::to_js_value;
use crate::ports::BoxFuture;
use crate::ports::command_runner::ShellKind;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// How a placeholder is treated, decided by the setting and the origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The command runs.
    Run,
    /// `disableSkillShellExecution` replaces it.
    Disabled,
    /// It reaches Claude as written.
    Literal,
}

const DISABLED_TEXT: &str = "[shell command execution disabled by policy]";

/// What a placeholder becomes when its command is not run.
type NotRun = fn(&Injection, &str) -> String;

/// `NOT_RUN[mode]`: what a placeholder becomes in a mode that does not run it.
fn not_run(mode: Mode) -> Option<NotRun> {
    match mode {
        Mode::Run => None,
        Mode::Disabled => Some(|_, _| DISABLED_TEXT.to_owned()),
        // "A `!` command line reaches Claude as literal text": as written, its arguments substituted.
        Mode::Literal => Some(|found, command| format!("{}{command}{}", found.open, found.close)),
    }
}

/// Whether `disableSkillShellExecution` replaces an origin's commands: it covers "skills and custom
/// commands from user, project, plugin, or additional-directory sources" and a synced skill ("or as
/// that placeholder when `disableSkillShellExecution` is on"); "Bundled and managed skills are not
/// affected". A bundled skill is the host's.
const fn disable_applies(kind: SkillOriginKind) -> bool {
    match kind {
        SkillOriginKind::Plugin => true,
        SkillOriginKind::Managed => false,
        SkillOriginKind::User => true,
        SkillOriginKind::Project => true,
        SkillOriginKind::Synced => true,
        SkillOriginKind::Bundled => false,
    }
}

/// `disableSkillShellExecution` replaces the commands of the origins it covers. Otherwise a synced
/// skill's commands are not run on the owner's machine, but are in an isolated session (cloud, Cowork).
#[must_use]
pub fn mode_of(skill: &Source, request: &ExpandRequest) -> Mode {
    let kind = skill.policy.origin.kind;
    if request.disable_shell_execution && disable_applies(kind) {
        return Mode::Disabled;
    }
    let isolated = request
        .session
        .as_ref()
        .and_then(|session| session.isolated_session);
    let runs = traits_of(kind).is_some_and(|traits| traits.runs_shell) || isolated == Some(true);
    if runs { Mode::Run } else { Mode::Literal }
}

/// Each placeholder's replacement, or the issue that stops the invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rendered {
    /// Each placeholder's replacement, in order.
    Replaced(Vec<String>),
    /// Why the invocation stops.
    Failed(PluginIssue),
}

/// One invocation's placeholders and what they are rendered with.
pub struct Run<'a> {
    /// The skill.
    pub skill: &'a Source,
    /// The request.
    pub request: &'a ExpandRequest,
    /// The ports.
    pub ports: ExpandPorts,
    /// The placeholders.
    pub injections: &'a [Injection],
    /// Each command with its variables substituted, the paths written in: what is checked.
    pub commands: Vec<String>,
    /// Each command with its arguments substituted only: what the shell's text is made from.
    pub templates: Vec<String>,
    /// The skill's variables.
    pub vars: &'a SkillVariables,
    /// The skill's grants.
    pub grants: Vec<String>,
}

/// One command's checked text, the shell's template and the variables its references read.
struct Texts {
    checked: String,
    template: String,
    environment: IndexMap<String, String>,
}

/// One command's checked text and the shell's template under the host's `shellArguments`: pasted,
/// the substitution's; bound, the authored command with its arguments bound, variables after.
async fn arguments_of(run: &Run<'_>, tool: SkillShell, found: &Injection, index: usize) -> Texts {
    let pasted = run.commands.get(index).unwrap_or(&found.command).clone();
    let template = run.templates.get(index).unwrap_or(&found.command).clone();
    let choice = run
        .request
        .session
        .as_ref()
        .and_then(|session| session.shell_arguments);
    if !binds_arguments(choice, tool) {
        let environment = IndexMap::new();
        return Texts {
            checked: pasted,
            template,
            environment,
        };
    }
    let input = ArgumentInput {
        raw_arguments: run.request.arguments.clone(),
        names: run.skill.own.arguments.clone(),
    };
    let bound = bind_shell_arguments(&found.command, &input);
    let checked = substitute_variables(&bound.checked, run.vars, "").await;
    Texts {
        checked: checked.text,
        template: bound.shell,
        environment: bound.environment,
    }
}

/// Each placeholder's command: the checked text, and the shell's under `shellPaths` and `shellArguments`.
async fn plan_of(run: &Run<'_>, tool: SkillShell) -> Vec<PlannedCommand> {
    let planned = run
        .injections
        .iter()
        .enumerate()
        .map(|(index, found)| -> BoxFuture<'_, _> {
            Box::pin(async move {
                let texts = arguments_of(run, tool, found, index).await;
                let shell = substitute_shell_command(&texts.template, run.vars, tool).await;
                let mut environment = shell.environment;
                environment.extend(texts.environment);
                PlannedCommand {
                    pattern: format!("{}{}{}", found.open, found.command, found.close),
                    command: texts.checked,
                    shell_command: shell.text,
                    environment,
                }
            })
        });
    promise_all(planned.collect()).await
}

/// The shell a `SkillShell` runs in.
const fn kind_of(tool: SkillShell) -> ShellKind {
    match tool {
        SkillShell::Bash => ShellKind::Bash,
        SkillShell::Powershell => ShellKind::Powershell,
    }
}

/// Runs the commands through the tool `resolveShell` chose.
async fn run_all(run: &Run<'_>, tool: SkillShell) -> Rendered {
    let Run {
        skill,
        request,
        ports,
        ..
    } = run;
    let commands = plan_of(run, tool).await;
    let cwd = match &request.cwd {
        Some(cwd) => cwd.clone(),
        None => ports.locations.project_dir().await,
    };
    let given: Option<IndexMap<String, String>> = match request.command_environment {
        Some(environment) => {
            environment
                .call(vec![to_js_value(&tool).unwrap_or_default()])
                .await
        }
        None => None,
    };
    let env = match given {
        Some(env) => env,
        None => ports.environment.snapshot().await,
    };
    let context = InjectionContext {
        name: format!("/{}", skill.runnable.command),
        shell: kind_of(tool),
        cwd,
        env,
        grants: run.grants.clone(),
        mode: request.permission_mode,
        forked_with_agent: skill.policy.context.as_deref() == Some("fork")
            && skill.runnable.agent.is_some(),
        signal: request.signal.clone(),
        bash_output_max_chars: request.bash_output_max_chars,
        bare: request.bare,
    };
    match run_injections(&commands, &context, ports.record).await {
        InjectionResults::Ran { outputs, .. } => Rendered::Replaced(outputs),
        InjectionResults::Failed { issue, .. } => Rendered::Failed(issue),
    }
}

/// The tool `resolveShell` named.
fn shell_of(tool: &str) -> SkillShell {
    if tool == "powershell" {
        SkillShell::Powershell
    } else {
        SkillShell::Bash
    }
}

/// The replacement of each placeholder. Judgment (spec/judgments.md): a skill with no placeholder to
/// run needs no shell, so it never fails for want of one.
pub async fn render(run: &Run<'_>, mode: Mode) -> Rendered {
    if let Some(make) = not_run(mode) {
        let commands = run.injections.iter().enumerate();
        return Rendered::Replaced(
            commands
                .map(|(i, found)| make(found, run.commands.get(i).map_or("", String::as_str)))
                .collect(),
        );
    }
    if run.injections.is_empty() {
        return Rendered::Replaced(vec![]);
    }
    let shell = resolve_shell(&run.skill.runnable, run.request.shells);
    match (shell.ok, shell.tool) {
        (true, Some(tool)) => run_all(run, shell_of(tool)).await,
        _ => {
            let name = format!("/{}", run.skill.runnable.command);
            Rendered::Failed(error(&name, shell.message.as_deref().unwrap_or_default()))
        }
    }
}
