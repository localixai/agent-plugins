//! Port of `src/skills/fork.ts`, switched.
//!
//! What a forked skill starts (skills.md, "Run skills in a subagent": "Claude Code starts a new
//! subagent of the type set in the `agent` field and gives it the skill content as its prompt. The
//! subagent doesn't see your conversation history"; the table: system prompt "From agent type", task
//! "SKILL.md content", also loads "CLAUDE.md, per the agent's startup context"). The agent catalog is
//! `agents`, a module of the same layer, so the host passes what it knows of an agent type in.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::js::JsValue;
use crate::registry::Callback;

/// What a subagent loads when it starts (sub-agents.md, "What loads at startup").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkStartContext {
    /// The user, project and local CLAUDE.md files.
    pub claude_md: bool,
    /// The git status snapshot.
    pub git_status: bool,
}

/// The fork variant of a skill's `Execution` (`skills/execution.ts`): what this module reads.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkExecution {
    /// The subagent type that runs the skill.
    pub agent: String,
    /// The subagent runs while the user keeps working; its result arrives when it completes.
    pub background: bool,
    /// "A backgrounded fork also runs with the narrower tool set": waiting keeps the full one.
    pub full_tool_set: bool,
    /// The subagent "doesn't see your conversation history".
    pub sees_conversation: bool,
}

/// The subagent type's system prompt is its type's, not the skill's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SystemPrompt {
    /// "From agent type".
    #[serde(rename = "agent-type")]
    AgentType,
}

/// What a forked skill starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkLaunch {
    /// The subagent type that runs the skill.
    pub agent: String,
    /// The skill's rendered content: the whole of what the subagent is asked to do.
    pub task: String,
    /// The subagent's system prompt is its type's, not the skill's.
    pub system_prompt: SystemPrompt,
    /// What the agent type loads beside the task: `Explore` and `Plan` start without CLAUDE.md and git status.
    pub loads: ForkStartContext,
    /// Runs in the background.
    pub background: bool,
    /// A foreground (waiting) fork keeps the full tool set; a background one gets the narrower set.
    pub full_tool_set: bool,
    /// Whether the subagent sees the conversation.
    pub sees_conversation: bool,
}

/// The launch, or why there is none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ForkResult {
    /// The launch.
    Launched {
        /// Always true.
        ok: bool,
        /// What starts.
        launch: ForkLaunch,
    },
    /// No such subagent.
    Refused {
        /// Always false.
        ok: bool,
        /// Why.
        issue: PluginIssue,
    },
}

/// The launch of a forked skill. Judgment (spec/judgments.md): an `agent` that is no built-in and no
/// custom agent of the session is an error naming it, not a silent fall-back to `general-purpose`.
pub async fn launch_fork(
    execution: &ForkExecution,
    command: &str,
    text: &str,
    agents: Callback,
) -> ForkResult {
    let loads = agents
        .call::<Option<ForkStartContext>>(vec![JsValue::from(execution.agent.as_str())])
        .await;
    let Some(loads) = loads else {
        return ForkResult::Refused {
            ok: false,
            issue: error(
                &format!("/{command}"),
                &format!(
                    "The skill runs in the subagent \"{}\", but no built-in or custom subagent has that name",
                    execution.agent
                ),
            ),
        };
    };
    ForkResult::Launched {
        ok: true,
        launch: ForkLaunch {
            agent: execution.agent.clone(),
            task: text.to_owned(),
            system_prompt: SystemPrompt::AgentType,
            loads,
            background: execution.background,
            full_tool_set: execution.full_tool_set,
            sees_conversation: execution.sees_conversation,
        },
    }
}

crate::registry::exports! {
    "skills/fork" {
        "launchFork" => async ok |execution: ForkExecution, command: String, text: String, agents: Callback|
            launch_fork(&execution, &command, &text, agents).await;
    }
}
