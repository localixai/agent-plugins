//! Port of `src/agents/workflow/limits.ts`, switched.
//!
//! The runtime constraints of a workflow run (workflows.md, "Behavior and limits"). Pure decisions over
//! counts the host holds: the kit runs no workflow and keeps no run state.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::js::{number, number_to_string};

/// "1,000 agents total per run | Prevents runaway loops".
const MAX_AGENTS_PER_RUN: f64 = 1000.0;

/// "Up to 4,096 items in a single `parallel()` or `pipeline()` call".
const MAX_FAN_OUT_ITEMS: f64 = 4096.0;

/// The calls that take a list of work (workflows.md, "What the saved script looks like").
pub const FAN_OUT_CALLS: [FanOutCall; 2] = [FanOutCall::Parallel, FanOutCall::Pipeline];

/// One of `FAN_OUT_CALLS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FanOutCall {
    /// `parallel()`.
    Parallel,
    /// `pipeline()`.
    Pipeline,
}

impl FanOutCall {
    /// The call as written.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Parallel => "parallel",
            Self::Pipeline => "pipeline",
        }
    }
}

/// Whether the run may start one more agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Admission {
    /// It may.
    Admitted {
        /// Always true.
        admitted: bool,
    },
    /// It may not.
    Refused {
        /// Always false.
        admitted: bool,
        /// Why.
        issue: PluginIssue,
    },
}

/// Whether the run may start one more agent, given how many `agent()` calls it has started. The
/// 1,000th is the last; the next call fails with an error (judgment: the docs name the cap and its
/// purpose, not the failure; a restart of an agent after a stall is the same agent and is not counted).
#[must_use]
pub fn admit_agent(started: f64) -> Admission {
    if started < MAX_AGENTS_PER_RUN {
        return Admission::Admitted { admitted: true };
    }
    let message = format!(
        "a workflow run starts at most {} agents; this run has started {}",
        number_to_string(MAX_AGENTS_PER_RUN),
        number_to_string(started)
    );
    Admission::Refused {
        admitted: false,
        issue: error("agent", &message),
    }
}

/// "the runtime rejects a longer list with an error": nothing in the list runs when it is too long.
#[must_use]
pub fn check_fan_out(call: FanOutCall, items: f64) -> Option<PluginIssue> {
    if items <= MAX_FAN_OUT_ITEMS {
        return None;
    }
    let message = format!(
        "{}() takes up to {} items; got {}",
        call.as_str(),
        number_to_string(MAX_FAN_OUT_ITEMS),
        number_to_string(items)
    );
    Some(error(call.as_str(), &message))
}

/// What can interrupt a run. "A run pauses on its own only for agent permission prompts and a
/// usage-limit wait": nothing else pauses it. `script-question` and `stage-sign-off` are the kit's own
/// names for the two kinds of mid-run input the docs turn away ("For sign-off between stages, run each
/// stage as its own workflow"), so a host can name why it refuses (spec/judgments.md).
pub const INTERRUPTIONS: [Interruption; 4] = [
    Interruption::AgentPermissionPrompt,
    Interruption::UsageLimitWait,
    Interruption::ScriptQuestion,
    Interruption::StageSignOff,
];

/// One of `INTERRUPTIONS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Interruption {
    /// An agent asks for a permission.
    AgentPermissionPrompt,
    /// The usage limit is hit.
    UsageLimitWait,
    /// The script asks a question.
    ScriptQuestion,
    /// A stage asks for a sign-off.
    StageSignOff,
}

const fn pauses_run(cause: Interruption) -> bool {
    match cause {
        Interruption::AgentPermissionPrompt | Interruption::UsageLimitWait => true,
        Interruption::ScriptQuestion | Interruption::StageSignOff => false,
    }
}

fn no_mid_run_input() -> PluginIssue {
    error(
        "script",
        "a workflow run takes no input while it runs; for sign-off between stages, run each stage as its own workflow",
    )
}

/// Whether the run waits for an interruption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Pause {
    /// The run waits.
    Pauses {
        /// Always true.
        pauses: bool,
    },
    /// The run turns it away.
    Refuses {
        /// Always false.
        pauses: bool,
        /// Why.
        issue: PluginIssue,
    },
}

/// Whether the run waits for it, or refuses it (judgment: the docs give no wording for the refusal).
#[must_use]
pub fn interrupt_run(cause: Interruption) -> Pause {
    if pauses_run(cause) {
        Pause::Pauses { pauses: true }
    } else {
        Pause::Refuses {
            pauses: false,
            issue: no_mid_run_input(),
        }
    }
}

/// The default and the bounds of `CLAUDE_CODE_WORKFLOW_MAX_CONCURRENT_AGENTS` (env-vars.md).
const DEFAULT_CONCURRENCY: f64 = 16.0;
const MAX_CONCURRENCY: f64 = 256.0;

/// How many agents a run executes at once: 16, "fewer when Claude Code has fewer CPUs available", or the
/// variable's value from 1 to 256. It "takes plain digits only; out-of-range values and other spellings
/// keep the default". Whether the CPU count also caps an explicit value is not said: it does not
/// (spec/judgments.md).
#[must_use]
pub fn workflow_concurrency(env: &HashMap<String, Option<String>>, cpus: f64) -> f64 {
    let asked = env
        .get("CLAUDE_CODE_WORKFLOW_MAX_CONCURRENT_AGENTS")
        .and_then(Option::as_deref);
    // `/^\d+$/`.
    let value = match asked {
        Some(text) if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) => {
            number(text)
        }
        Some(_) | None => 0.0,
    };
    if (1.0..=MAX_CONCURRENCY).contains(&value) {
        return value;
    }
    let floored = cpus.floor();
    if floored.is_nan() {
        return f64::NAN;
    }
    DEFAULT_CONCURRENCY.min(floored).max(1.0)
}

crate::registry::exports! {
    "agents/workflow/limits" {
        "FAN_OUT_CALLS" => ok || FAN_OUT_CALLS;
        "admitAgent" => ok |started: f64| admit_agent(started);
        "checkFanOut" => ok |call: FanOutCall, items: f64| check_fan_out(call, items);
        "INTERRUPTIONS" => ok || INTERRUPTIONS;
        "interruptRun" => ok |cause: Interruption| interrupt_run(cause);
        "workflowConcurrency" => ok |env: HashMap<String, Option<String>>, cpus: f64| workflow_concurrency(&env, cpus);
    }
}
