//! Port of `src/skills/execution.ts`, switched.
//!
//! How a skill runs once it is invoked (skills.md, "Run skills in a subagent", "Frontmatter
//! reference", "How injected commands run"; model-config.md, "Restrict model selection" and "Adjust
//! effort level"). Pure over the skill and what the host knows of the session; the host reads its
//! environment through `readExecutionEnvironment`.

use serde::{Deserialize, Serialize};

use crate::format::env_flag::is_truthy_env;
use crate::js::JsValue;
use crate::ports::environment::Environment;
use crate::protocol::effort::EFFORT_LEVELS;
use crate::registry::{Port, Set};

/// What of a skill's frontmatter decides how it runs (`Runnable`, with its `command` and `name`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Runnable {
    /// `agent`: the subagent type for `context: fork`.
    #[serde(default)]
    pub agent: Option<String>,
    /// `background`, default `true`.
    #[serde(default)]
    pub background: bool,
    /// `context`: `fork` runs the skill in a forked subagent.
    #[serde(default)]
    pub context: JsValue,
    /// `effort`: one of the five documented levels.
    #[serde(default)]
    pub effort: Option<String>,
    /// `model`, as written.
    #[serde(default)]
    pub model: Option<String>,
    /// `shell`, default `bash`.
    #[serde(default)]
    pub shell: Option<String>,
    /// Whether the frontmatter set `shell`.
    #[serde(default)]
    pub shell_declared: Option<bool>,
    /// What the user types after `/`.
    #[serde(default)]
    pub command: String,
    /// The skill's name.
    #[serde(default)]
    pub name: String,
}

/// The documented default agent: "If omitted, uses `general-purpose`".
const DEFAULT_AGENT: &str = "general-purpose";

/// What fires a skill; the last two always wait for a forked skill's result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    /// The user.
    User,
    /// The model.
    Model,
    /// A scheduled task.
    ScheduledTask,
    /// A workflow's agent.
    WorkflowAgent,
}

/// What the host knows of the session a skill is forked in.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContext {
    /// False with `-p` or through the Agent SDK.
    pub interactive: bool,
    /// `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` is `1`.
    pub background_tasks_disabled: bool,
    /// What fired the skill.
    pub trigger: Trigger,
    /// The commands of forked skills still running.
    #[serde(default)]
    pub running_forks: Option<Set<String>>,
}

/// Why a forked skill waits for its result instead of running in the background.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WaitReason {
    /// `background: false`.
    BackgroundFalse,
    /// Not interactive.
    NonInteractive,
    /// Background tasks are disabled.
    BackgroundTasksDisabled,
    /// The same skill already runs.
    AlreadyRunning,
    /// A scheduled task fired it.
    ScheduledTask,
    /// A workflow's agent fired it.
    WorkflowAgent,
}

/// In the order skills.md gives them: the field first, then the cases of the list, then the workflow note.
const WAIT_ORDER: [WaitReason; 6] = [
    WaitReason::BackgroundFalse,
    WaitReason::NonInteractive,
    WaitReason::BackgroundTasksDisabled,
    WaitReason::AlreadyRunning,
    WaitReason::ScheduledTask,
    WaitReason::WorkflowAgent,
];

/// The triggers that always wait for a forked skill's result, and why.
const fn trigger_wait(trigger: Trigger) -> Option<WaitReason> {
    match trigger {
        Trigger::User | Trigger::Model => None,
        Trigger::ScheduledTask => Some(WaitReason::ScheduledTask),
        Trigger::WorkflowAgent => Some(WaitReason::WorkflowAgent),
    }
}

fn wait_check(reason: WaitReason, skill: &Runnable, ctx: &ExecutionContext) -> bool {
    match reason {
        WaitReason::BackgroundFalse => !skill.background,
        WaitReason::NonInteractive => !ctx.interactive,
        WaitReason::BackgroundTasksDisabled => ctx.background_tasks_disabled,
        WaitReason::AlreadyRunning => ctx
            .running_forks
            .as_ref()
            .is_some_and(|forks| forks.members.contains(&skill.command)),
        WaitReason::ScheduledTask | WaitReason::WorkflowAgent => {
            trigger_wait(ctx.trigger) == Some(reason)
        }
    }
}

/// Inline, or a forked subagent and whether the caller waits for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Execution {
    /// In the conversation.
    Inline,
    /// In a forked subagent.
    #[serde(rename_all = "camelCase")]
    Fork {
        /// The subagent type that runs the skill.
        agent: String,
        /// The subagent runs while the user keeps working; its result arrives when it completes.
        background: bool,
        /// Why it does not.
        waits_for: Vec<WaitReason>,
        /// "A backgrounded fork also runs with the narrower tool set": waiting keeps the full one.
        full_tool_set: bool,
        /// The subagent "doesn't see your conversation history".
        sees_conversation: bool,
    },
}

/// Inline, or a forked subagent and whether the caller waits for it.
#[must_use]
pub fn resolve_execution(skill: &Runnable, ctx: &ExecutionContext) -> Execution {
    if skill.context.is_undefined() {
        return Execution::Inline;
    }
    let waits_for: Vec<WaitReason> = WAIT_ORDER
        .into_iter()
        .filter(|reason| wait_check(*reason, skill, ctx))
        .collect();
    let background = waits_for.is_empty();
    Execution::Fork {
        agent: skill
            .agent
            .clone()
            .unwrap_or_else(|| DEFAULT_AGENT.to_owned()),
        background,
        waits_for,
        full_tool_set: !background,
        sees_conversation: false,
    }
}

/// The two variables a skill's execution reads (env-vars.md).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionEnvironment {
    /// `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS`.
    pub background_tasks_disabled: bool,
    /// `CLAUDE_CODE_EFFORT_LEVEL`: a level, or `none` for `auto`/`unset` (no level is sent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

fn effort_level_of(value: &str) -> Option<String> {
    EFFORT_LEVELS
        .iter()
        .find(|level| **level == value)
        .map(|level| (*level).to_owned())
}

/// `auto` and `unset` "do the same thing": Claude Code sends no effort level (model-config.md).
const NO_LEVEL: [&str; 2] = ["auto", "unset"];

/// The two variables a skill's execution reads, from the host's environment.
pub async fn read_execution_environment(environment: &dyn Environment) -> ExecutionEnvironment {
    let value = environment.get("CLAUDE_CODE_EFFORT_LEVEL").await;
    let effort = value.as_deref().and_then(read_effort_value);
    let disabled = environment
        .get("CLAUDE_CODE_DISABLE_BACKGROUND_TASKS")
        .await;
    ExecutionEnvironment {
        background_tasks_disabled: is_truthy_env(disabled.as_deref()),
        effort,
    }
}

/// Judgment (spec/judgments.md): a value that is neither a level nor `auto`/`unset` is ignored.
fn read_effort_value(value: &str) -> Option<String> {
    if NO_LEVEL.contains(&value) {
        return Some("none".to_owned());
    }
    effort_level_of(value)
}

mod turn;
pub use turn::{
    EffortChoice, EffortSource, ModelKept, Target, TurnOverrides, TurnSession,
    resolve_turn_overrides,
};

/// Which shells the host has.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ShellAvailability {
    /// Bash is available (not Windows without Git Bash).
    pub bash: bool,
    /// The PowerShell tool is enabled.
    pub powershell: bool,
}

/// The tool that runs a skill's commands, or the failure that stops the invocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShellResolution {
    /// Whether a tool runs them.
    pub ok: bool,
    /// The tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<&'static str>,
    /// Why none does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

const fn tool(tool: &'static str) -> ShellResolution {
    ShellResolution {
        ok: true,
        tool: Some(tool),
        message: None,
    }
}

/// The rules of "How injected commands run", in the order the page gives them.
fn shell_rule(rule: usize, skill: &Runnable, tools: ShellAvailability) -> Option<ShellResolution> {
    let shell = skill.shell.as_deref();
    match rule {
        0 if shell == Some("powershell") && tools.powershell => Some(tool("powershell")),
        1 if skill.shell_declared == Some(true) && shell == Some("bash") && !tools.bash => {
            Some(ShellResolution {
                ok: false,
                tool: None,
                message: Some(format!(
                    "Skill {} requires bash (`shell: bash` in frontmatter) but Git Bash was not found",
                    skill.name
                )),
            })
        }
        2 if tools.bash => Some(tool("bash")),
        3 if tools.powershell => Some(tool("powershell")),
        _ => None,
    }
}

/// Which tool runs a skill's `` !`command` `` blocks, or the one failure that stops the invocation.
/// Judgment (spec/judgments.md): with neither tool the docs name no message; the invocation fails
/// naming the skill.
#[must_use]
pub fn resolve_shell(skill: &Runnable, tools: ShellAvailability) -> ShellResolution {
    (0..4)
        .find_map(|rule| shell_rule(rule, skill, tools))
        .unwrap_or_else(|| ShellResolution {
            ok: false,
            tool: None,
            message: Some(format!(
                "Skill {} cannot run its shell commands: neither Bash nor the PowerShell tool is available",
                skill.name
            )),
        })
}

crate::registry::exports! {
    "skills/execution" {
        "resolveExecution" => ok |skill: Runnable, ctx: ExecutionContext| resolve_execution(&skill, &ctx);
        "readExecutionEnvironment" => async ok |environment: Port| read_execution_environment(&environment).await;
        "resolveTurnOverrides" => async ok |skill: Runnable, session: TurnSession|
            resolve_turn_overrides(&skill, &session).await;
        "resolveShell" => ok |skill: Runnable, tools: ShellAvailability| resolve_shell(&skill, tools);
    }
}
