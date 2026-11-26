//! Port of `src/agents/workflow/replay.ts`, switched.
//!
//! Which agents run again when a workflow run is relaunched (workflows.md, "Resume after a pause",
//! "Which agents run again"): "Claude Code replays the run in the order agents start, and each agent
//! either returns its saved result or runs again". Pure over the saved record of the earlier run and the
//! calls the script starts now, one at a time, because a rerun agent may return something else and
//! change the prompts after it. A call is its prompt and its options: the SDK's `resumeFromRunId` says
//! "Completed agent() calls with unchanged (prompt, opts) return their cached results".

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::js::indexing::element_at;

/// How an agent of the earlier run ended.
pub const SAVED_STATUSES: [SavedStatus; 3] = [
    SavedStatus::Completed,
    SavedStatus::Failed,
    SavedStatus::Running,
];

/// One of `SAVED_STATUSES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SavedStatus {
    /// It finished.
    Completed,
    /// It failed.
    Failed,
    /// It had not finished when the run was stopped.
    Running,
}

/// One agent of the earlier run, in the order it started.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SavedAgent {
    /// Its prompt.
    pub prompt: String,
    /// A fingerprint of the call's other options (`schema`, `label`, `model` and the rest of `opts`),
    /// which the host computes the same way for the saved record and for a call it starts now; absent
    /// when the host keeps none, then only the prompt is compared (judgment).
    #[serde(default)]
    pub options: Option<String>,
    /// How it ended.
    pub status: SavedStatus,
    /// Its result, for a completed agent.
    #[serde(default)]
    pub result: JsValue,
}

/// An `agent()` call the script starts now: its prompt and the fingerprint of its options.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AgentCall {
    /// Its prompt.
    pub prompt: String,
    /// The fingerprint of its options.
    #[serde(default)]
    pub options: Option<String>,
}

/// Why an agent runs again.
pub const RERUN_CAUSES: [RerunCause; 7] = [
    RerunCause::PromptDiffers,
    RerunCause::OptionsDiffer,
    RerunCause::AfterChange,
    RerunCause::Failed,
    RerunCause::AfterFailed,
    RerunCause::WasRunning,
    RerunCause::NotSaved,
];

/// One of `RERUN_CAUSES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RerunCause {
    /// Its prompt changed.
    PromptDiffers,
    /// Its options changed.
    OptionsDiffer,
    /// An agent before it changed.
    AfterChange,
    /// It failed.
    Failed,
    /// An agent before it failed.
    AfterFailed,
    /// It was still running when the run was stopped.
    WasRunning,
    /// The earlier run has no agent for this call.
    NotSaved,
}

/// What the script's call gets: the saved result, or a rerun.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Replay {
    /// The saved result.
    Saved {
        /// Always true.
        saved: bool,
        /// The saved result.
        result: JsValue,
    },
    /// The agent runs again.
    Rerun {
        /// Always false.
        saved: bool,
        /// Why.
        cause: RerunCause,
    },
}

/// Why everything from here runs again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cascade {
    /// A completed agent changed.
    AfterChange,
    /// An agent failed.
    AfterFailed,
}

/// Where the replay is: the next saved agent, and whether everything from here runs again, and why.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayState {
    /// The index of the next saved agent.
    pub next: f64,
    /// Why everything from here runs again, when it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cascade: Option<Cascade>,
}

/// Where a replay starts.
pub const REPLAY_START: ReplayState = ReplayState {
    next: 0.0,
    cascade: None,
};

struct Step {
    replay: Replay,
    cascade: Option<Cascade>,
}

const fn rerun(cause: RerunCause, cascade: Option<Cascade>) -> Step {
    Step {
        replay: Replay::Rerun {
            saved: false,
            cause,
        },
        cascade,
    }
}

/// One rule per status of the saved agent:
/// - completed: its saved result when the prompt and the options are the same; the first one whose
///   prompt differs "runs again, and so does every agent after it, even ones that completed" (judgment:
///   changed options count the same, as the SDK's `(prompt, opts)` says);
/// - failed: runs again, and so does every agent after it;
/// - still running when you stopped: "starts over". Stopping the whole run does not count it as failed,
///   so the agents after it are not dragged along unless its prompt changed (judgment: the docs say the
///   rule for a changed prompt only of completed agents; it is applied here too).
fn rule(record: &SavedAgent, call: &AgentCall) -> Step {
    match record.status {
        SavedStatus::Completed => {
            if record.prompt != call.prompt {
                return rerun(RerunCause::PromptDiffers, Some(Cascade::AfterChange));
            }
            if record.options != call.options {
                return rerun(RerunCause::OptionsDiffer, Some(Cascade::AfterChange));
            }
            Step {
                replay: Replay::Saved {
                    saved: true,
                    result: record.result.clone(),
                },
                cascade: None,
            }
        }
        SavedStatus::Failed => rerun(RerunCause::Failed, Some(Cascade::AfterFailed)),
        SavedStatus::Running => rerun(
            RerunCause::WasRunning,
            if record.prompt == call.prompt && record.options == call.options {
                None
            } else {
                Some(Cascade::AfterChange)
            },
        ),
    }
}

/// What the replay did with one call, and where it stands after.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Replayed {
    /// What the call gets.
    pub replay: Replay,
    /// The replay's state after it.
    pub state: ReplayState,
}

const fn cascade_cause(cascade: Cascade) -> RerunCause {
    match cascade {
        Cascade::AfterChange => RerunCause::AfterChange,
        Cascade::AfterFailed => RerunCause::AfterFailed,
    }
}

/// What the agent the script starts now does: returns its saved result or runs again.
#[must_use]
pub fn replay_agent(saved: &[SavedAgent], state: ReplayState, call: &AgentCall) -> Replayed {
    let next = state.next + 1.0;
    let record = element_at(saved, state.next);
    if let Some(cascade) = state.cascade {
        return Replayed {
            replay: Replay::Rerun {
                saved: false,
                cause: cascade_cause(cascade),
            },
            state: ReplayState {
                next,
                cascade: state.cascade,
            },
        };
    }
    let Some(record) = record else {
        return Replayed {
            replay: Replay::Rerun {
                saved: false,
                cause: RerunCause::NotSaved,
            },
            state: ReplayState {
                next,
                cascade: None,
            },
        };
    };
    let step = rule(record, call);
    Replayed {
        replay: step.replay,
        state: ReplayState {
            next,
            cascade: step.cascade,
        },
    }
}

/// The decisions for a script that starts `calls` in this order, when no rerun agent returns anything
/// different from before (what a relaunch of an unchanged or edited-prompt script shows).
#[must_use]
pub fn plan_replay(saved: &[SavedAgent], calls: &[AgentCall]) -> Vec<Replay> {
    let mut state = REPLAY_START;
    calls
        .iter()
        .map(|call| {
            let step = replay_agent(saved, state, call);
            state = step.state;
            step.replay
        })
        .collect()
}

/// What stopping can stop.
pub const STOP_SCOPES: [StopScope; 2] = [StopScope::WholeRun, StopScope::OneAgent];

/// One of `STOP_SCOPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StopScope {
    /// The whole run.
    WholeRun,
    /// One agent, by selecting it in `/workflows` and pressing `x`.
    OneAgent,
}

/// How the saved record keeps an agent that had not finished when it was stopped: "Stopping the whole
/// run doesn't count any agent as failed"; "Stopping one agent alone, by selecting it in `/workflows`
/// and pressing `x`, counts as failing."
pub const RECORDED_WHEN_STOPPED: RecordedWhenStopped = RecordedWhenStopped {
    whole_run: SavedStatus::Running,
    one_agent: SavedStatus::Failed,
};

/// The table of `recorded_when_stopped`, keyed by the scope's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RecordedWhenStopped {
    /// Stopping the whole run.
    pub whole_run: SavedStatus,
    /// Stopping one agent.
    pub one_agent: SavedStatus,
}

crate::registry::exports! {
    "agents/workflow/replay" {
        "SAVED_STATUSES" => ok || SAVED_STATUSES;
        "RERUN_CAUSES" => ok || RERUN_CAUSES;
        "REPLAY_START" => ok || REPLAY_START;
        "replayAgent" => ok |saved: Vec<SavedAgent>, state: ReplayState, call: AgentCall| replay_agent(&saved, state, &call);
        "planReplay" => ok |saved: Vec<SavedAgent>, calls: Vec<AgentCall>| plan_replay(&saved, &calls);
        "STOP_SCOPES" => ok || STOP_SCOPES;
        "RECORDED_WHEN_STOPPED" => ok || RECORDED_WHEN_STOPPED;
    }
}
