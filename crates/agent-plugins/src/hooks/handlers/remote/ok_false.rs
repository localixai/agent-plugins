//! Port of `src/hooks/handlers/remote/ok-false.ts`, switched.
//!
//! What `ok: false` of a `prompt` or `agent` hook does on each event (hooks.md, "Response schema":
//! "What happens on `ok: false` depends on the event"). The run says what the model decided; this
//! says what the event does with it, as data the interpreter turns into its outcome.

use serde::{Deserialize, Serialize};

use super::types::ModelRun;

/// The events that run `prompt` and `agent` hooks (`PermissionRequest`: prompt only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptEvent {
    /// `PermissionDenied`
    PermissionDenied,
    /// `PermissionRequest`
    PermissionRequest,
    /// `PostToolBatch`
    PostToolBatch,
    /// `PostToolUse`
    PostToolUse,
    /// `PostToolUseFailure`
    PostToolUseFailure,
    /// `PreToolUse`
    PreToolUse,
    /// `Stop`
    Stop,
    /// `SubagentStop`
    SubagentStop,
    /// `TaskCompleted`
    TaskCompleted,
    /// `TaskCreated`
    TaskCreated,
    /// `TeammateIdle`
    TeammateIdle,
    /// `UserPromptExpansion`
    UserPromptExpansion,
    /// `UserPromptSubmit`
    UserPromptSubmit,
}

/// The events of `OK_FALSE_RULES`, in the table's order.
pub const PROMPT_EVENTS: [PromptEvent; 13] = [
    PromptEvent::Stop,
    PromptEvent::SubagentStop,
    PromptEvent::PreToolUse,
    PromptEvent::PostToolUse,
    PromptEvent::PostToolBatch,
    PromptEvent::UserPromptSubmit,
    PromptEvent::UserPromptExpansion,
    PromptEvent::PostToolUseFailure,
    PromptEvent::TaskCreated,
    PromptEvent::TaskCompleted,
    PromptEvent::TeammateIdle,
    PromptEvent::PermissionRequest,
    PromptEvent::PermissionDenied,
];

/// Where the reason goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Delivery {
    /// "Fed back to Claude as its next instruction".
    ClaudeInstruction,
    /// "Returned to Claude as a tool error".
    ClaudeToolError,
    /// "The reason appears in the chat as a warning line".
    UserWarning,
    /// "Fed back to the teammate".
    Teammate,
}

/// What the verdict does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum OkFalseEffect {
    /// No effect: `ok: true`, or an event on which `ok: false` changes nothing.
    None,
    /// `Stop` and `SubagentStop` with `impossible: true`: "Claude Code allows the stop and the
    /// turn ends".
    AllowStop,
    /// A block.
    Block {
        /// The reason.
        reason: String,
        /// The turn ends (or the teammate stops) instead of going on.
        ends_turn: bool,
        /// Where the reason goes.
        delivery: Delivery,
        /// `PreToolUse`: "the tool call is denied".
        denies_tool: bool,
    },
}

/// What a rule reads: the verdict, and how the hook was configured.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkFalseCase {
    /// The reason.
    pub reason: String,
    /// `continueOnBlock: true` of a prompt hook, or an agent hook (it behaves as if set).
    pub continues: bool,
    /// The model said `impossible` and the hook type reads it (prompt hooks only).
    pub impossible: bool,
    /// `TaskCompleted` fired because a teammate stopped, not because a task was marked completed.
    pub teammate_stopped: bool,
}

fn block(
    found: &OkFalseCase,
    ends_turn: bool,
    delivery: Delivery,
    denies_tool: bool,
) -> OkFalseEffect {
    OkFalseEffect::Block {
        reason: found.reason.clone(),
        ends_turn,
        delivery,
        denies_tool,
    }
}

/// "Fed back to Claude ... and the turn continues", or with `continueOnBlock` unset "the turn
/// ends".
fn ends_unless_continues(
    found: &OkFalseCase,
    delivery: Delivery,
    denies_tool: bool,
) -> OkFalseEffect {
    if found.continues {
        block(found, false, delivery, denies_tool)
    } else {
        block(found, true, Delivery::UserWarning, denies_tool)
    }
}

fn always_ends(found: &OkFalseCase) -> OkFalseEffect {
    block(found, true, Delivery::UserWarning, false)
}

fn always_tool_error(found: &OkFalseCase) -> OkFalseEffect {
    block(found, false, Delivery::ClaudeToolError, false)
}

/// `TaskCompleted`: "when it fires because a task is marked completed during a turn, the reason is
/// returned to Claude as a tool error and the turn continues, regardless of `continueOnBlock`. When
/// it fires because a teammate stops, it behaves like `TeammateIdle`".
fn task_completed(found: &OkFalseCase) -> OkFalseEffect {
    if found.teammate_stopped {
        ends_unless_continues(found, Delivery::Teammate, false)
    } else {
        always_tool_error(found)
    }
}

/// What a prompt or agent hook's `ok: false` does on `event` (`OK_FALSE_RULES[event]`).
#[must_use]
pub fn ok_false_rule(event: PromptEvent, found: &OkFalseCase) -> OkFalseEffect {
    match event {
        PromptEvent::Stop | PromptEvent::SubagentStop => {
            if found.impossible {
                OkFalseEffect::AllowStop
            } else {
                block(found, false, Delivery::ClaudeInstruction, false)
            }
        }
        PromptEvent::PreToolUse => ends_unless_continues(found, Delivery::ClaudeToolError, true),
        PromptEvent::PostToolUse => {
            ends_unless_continues(found, Delivery::ClaudeInstruction, false)
        }
        PromptEvent::PostToolBatch
        | PromptEvent::UserPromptSubmit
        | PromptEvent::UserPromptExpansion => always_ends(found),
        PromptEvent::PostToolUseFailure | PromptEvent::TaskCreated => always_tool_error(found),
        PromptEvent::TaskCompleted => task_completed(found),
        PromptEvent::TeammateIdle => ends_unless_continues(found, Delivery::Teammate, false),
        // "`ok: false` has no effect" (PermissionRequest); "the denial already happened"
        // (PermissionDenied).
        PromptEvent::PermissionRequest | PromptEvent::PermissionDenied => OkFalseEffect::None,
    }
}

/// Whether `event` is one of `OK_FALSE_RULES`.
#[must_use]
pub fn is_prompt_event(event: &str) -> bool {
    parse_event(event).is_some()
}

fn parse_event(event: &str) -> Option<PromptEvent> {
    PROMPT_EVENTS
        .iter()
        .copied()
        .find(|known| name(*known) == event)
}

const fn name(event: PromptEvent) -> &'static str {
    match event {
        PromptEvent::PermissionDenied => "PermissionDenied",
        PromptEvent::PermissionRequest => "PermissionRequest",
        PromptEvent::PostToolBatch => "PostToolBatch",
        PromptEvent::PostToolUse => "PostToolUse",
        PromptEvent::PostToolUseFailure => "PostToolUseFailure",
        PromptEvent::PreToolUse => "PreToolUse",
        PromptEvent::Stop => "Stop",
        PromptEvent::SubagentStop => "SubagentStop",
        PromptEvent::TaskCompleted => "TaskCompleted",
        PromptEvent::TaskCreated => "TaskCreated",
        PromptEvent::TeammateIdle => "TeammateIdle",
        PromptEvent::UserPromptExpansion => "UserPromptExpansion",
        PromptEvent::UserPromptSubmit => "UserPromptSubmit",
    }
}

/// How a prompt or agent hook was configured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkFalseOptions {
    /// The hook is an `agent` hook: "the way it handles a prompt hook with `continueOnBlock:
    /// true`"; no `impossible`.
    pub agent: bool,
    /// `continueOnBlock` of the prompt hook.
    #[serde(default)]
    pub continue_on_block: Option<bool>,
    /// `TaskCompleted` only: a teammate stopping fired it. Without it the event is a
    /// `TaskUpdate`.
    #[serde(default)]
    pub teammate_stopped: Option<bool>,
}

/// What the verdict of a prompt or agent hook does on `event`.
#[must_use]
pub fn ok_false_effect(
    event: PromptEvent,
    run: &ModelRun,
    options: OkFalseOptions,
) -> OkFalseEffect {
    if run.ok {
        return OkFalseEffect::None;
    }
    ok_false_rule(
        event,
        &OkFalseCase {
            reason: run.reason.clone().unwrap_or_default(),
            continues: options.agent || options.continue_on_block == Some(true),
            impossible: !options.agent && run.impossible == Some(true),
            teammate_stopped: options.teammate_stopped == Some(true),
        },
    )
}

crate::registry::exports! {
    "hooks/handlers/remote/ok-false" {
        "OK_FALSE_RULES" => ok || PROMPT_EVENTS;
        "okFalseRule" => ok |event: PromptEvent, found: OkFalseCase| ok_false_rule(event, &found);
        "isPromptEvent" => ok |event: String| is_prompt_event(&event);
        "okFalseEffect" => ok |event: PromptEvent, run: ModelRun, options: OkFalseOptions| ok_false_effect(event, &run, options);
    }
}
