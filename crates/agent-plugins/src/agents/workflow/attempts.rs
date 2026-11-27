//! Port of `src/agents/workflow/attempts.ts`, switched.
//!
//! When an agent stalls and restarts (workflows.md, "When an agent stalls and restarts"): an agent
//! whose output stops arriving starts over from the same prompt, at most five times, so at most six
//! attempts. Pure decisions over the endings of the attempts so far; the host measures time and runs
//! the agent.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::format::env_number::positive_whole_env;
use crate::format::issues::{PluginIssue, error};
use crate::js::number_to_string;

/// "An agent restarts at most five times, counting any restart you ask for with `r`".
const MAX_RESTARTS: usize = 5;
const MAX_ATTEMPTS: usize = MAX_RESTARTS + 1;

/// How an attempt ended without finishing: the whole stall window without output, a response stream that went silent, or a restart asked for.
pub const ATTEMPT_ENDINGS: [AttemptEnding; 3] = [
    AttemptEnding::Stalled,
    AttemptEnding::LostReply,
    AttemptEnding::Restarted,
];

/// One of `ATTEMPT_ENDINGS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttemptEnding {
    /// The whole stall window went without output.
    Stalled,
    /// A response stream went silent.
    LostReply,
    /// A restart was asked for.
    Restarted,
}

/// How an ending reads in the list of an `abandoned` error (judgment for the last two).
const fn ending_words(ending: AttemptEnding) -> &'static str {
    match ending {
        AttemptEnding::Stalled => "stalled",
        AttemptEnding::LostReply => "lost its reply",
        AttemptEnding::Restarted => "restarted on request",
    }
}

/// What the detail line shows for the ending of the previous attempt: `attempt 2 (stalled)`.
const fn ending_detail(ending: AttemptEnding) -> &'static str {
    match ending {
        AttemptEnding::Stalled => "stalled",
        AttemptEnding::LostReply => "lost reply",
        AttemptEnding::Restarted => "restarted",
    }
}

/// The `/workflows` labels of an attempt: `(retry 1)` after the name, `attempt 2 (stalled)` in the detail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptLabels {
    /// After the attempt's name.
    pub name_suffix: String,
    /// In the detail line.
    pub detail: String,
}

/// The agent starts over.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Restart {
    /// Always true.
    pub restart: bool,
    /// The number of the attempt that starts, from 2.
    pub attempt: f64,
    /// Its labels.
    pub labels: AttemptLabels,
}

/// Why the `agent()` call failed, and the error whose start says why.
pub const FAILURE_CAUSES: [WorkflowFailureCause; 3] = [
    WorkflowFailureCause::Stalled,
    WorkflowFailureCause::LostReply,
    WorkflowFailureCause::Abandoned,
];

/// One of `FAILURE_CAUSES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowFailureCause {
    /// Every attempt stalled.
    Stalled,
    /// Every attempt lost its reply.
    LostReply,
    /// The attempts ended in different ways.
    Abandoned,
}

/// The `agent()` call failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentFailure {
    /// Always false.
    pub restart: bool,
    /// Why.
    pub cause: WorkflowFailureCause,
    /// The error.
    pub issue: PluginIssue,
}

/// What follows an attempt: a restart, or the failure.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum AttemptVerdict {
    /// The agent starts over.
    Restart(Restart),
    /// The call fails.
    Failure(AgentFailure),
}

fn attempts_word() -> String {
    format!("{MAX_ATTEMPTS} attempts")
}

/// The start of each error is the docs' wording; what follows is the advice of the same bullet.
fn message_of(cause: WorkflowFailureCause, endings: &[AttemptEnding]) -> String {
    let attempts_word = attempts_word();
    match cause {
        WorkflowFailureCause::Stalled => format!(
            "agent stalled on all {attempts_word}: every attempt went the whole window without output; if the agent's work keeps it silent that long, lengthen the window"
        ),
        WorkflowFailureCause::LostReply => format!(
            "agent lost its reply on all {attempts_word}: every attempt's response stream went silent and Claude Code gave up waiting on it; lengthening the stall window does not help, CLAUDE_STREAM_IDLE_TIMEOUT_MS sets the streaming idle watchdog's timeout"
        ),
        WorkflowFailureCause::Abandoned => format!(
            "agent abandoned after {attempts_word}: {}",
            endings
                .iter()
                .map(|ending| ending_words(*ending))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn same_ending(endings: &[AttemptEnding]) -> Option<AttemptEnding> {
    let first = endings.first()?;
    endings.iter().all(|each| each == first).then_some(*first)
}

/// Every attempt stalled, every one lost its reply, or they ended in different ways.
fn failure_cause(endings: &[AttemptEnding]) -> WorkflowFailureCause {
    match same_ending(endings) {
        Some(AttemptEnding::Stalled) => WorkflowFailureCause::Stalled,
        Some(AttemptEnding::LostReply) => WorkflowFailureCause::LostReply,
        Some(AttemptEnding::Restarted) | None => WorkflowFailureCause::Abandoned,
    }
}

/// The labels of attempt `attempt` (from 1), which follows an attempt that ended as `previous`.
#[must_use]
pub fn attempt_labels(attempt: f64, previous: Option<AttemptEnding>) -> AttemptLabels {
    let Some(previous) = previous.filter(|_| attempt > 1.0) else {
        return AttemptLabels {
            name_suffix: String::new(),
            detail: String::new(),
        };
    };
    AttemptLabels {
        name_suffix: format!(" (retry {})", number_to_string(attempt - 1.0)),
        detail: format!(
            "attempt {} ({})",
            number_to_string(attempt),
            ending_detail(previous)
        ),
    }
}

/// What follows an attempt that ended without finishing. `endings` holds how every attempt so far
/// ended, the one just ended last. Up to the fifth attempt the agent starts over; when the sixth ends the
/// same way the `agent()` call fails.
#[must_use]
pub fn after_attempt(endings: &[AttemptEnding]) -> AttemptVerdict {
    if endings.len() < MAX_ATTEMPTS {
        let attempt = f64::from(u32::try_from(endings.len() + 1).unwrap_or(u32::MAX));
        return AttemptVerdict::Restart(Restart {
            restart: true,
            attempt,
            labels: attempt_labels(attempt, endings.last().copied()),
        });
    }
    let counted = endings.get(..MAX_ATTEMPTS).unwrap_or(endings);
    let cause = failure_cause(counted);
    AttemptVerdict::Failure(AgentFailure {
        restart: false,
        cause,
        issue: error("agent", &message_of(cause, counted)),
    })
}

/// Whether a restart asked for on attempt `attempt` is allowed: it counts toward the five.
#[must_use]
pub fn can_restart_on_request(attempt: f64) -> bool {
    attempt < f64::from(u32::try_from(MAX_ATTEMPTS).unwrap_or(u32::MAX))
}

/// "Pass `stallMs` in milliseconds on its `agent()` call", else the environment variable, else the default (env-vars.md).
const DEFAULT_STALL_MS: f64 = 600_000.0;
const STALL_ENV: &str = "CLAUDE_ASYNC_AGENT_STALL_TIMEOUT_MS";

/// J3.17: read as every numeric variable is (`2e3`, `64_000`).
fn positive_integer(text: Option<&str>) -> Option<f64> {
    positive_whole_env(text)
}

/// What the stall window is worked out from.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StallWindowInput {
    /// `stallMs` passed on this agent's `agent()` call.
    #[serde(default)]
    pub stall_ms: Option<f64>,
    /// The default the host works out when `CLAUDE_STREAM_IDLE_TIMEOUT_MS` is raised while the stream
    /// watchdog is on ("the default rises with it"); `DEFAULT_STALL_MS` when absent. Only the default
    /// rises: the call's `stallMs` and the environment variable still win over it.
    #[serde(default)]
    pub default_ms: Option<f64>,
    /// The environment.
    pub env: HashMap<String, Option<String>>,
}

/// `Number.isInteger(value) && value > 0`.
fn is_positive_integer(value: Option<f64>) -> Option<f64> {
    value.filter(|number| number.is_finite() && number.fract() == 0.0 && *number > 0.0)
}

/// How long Claude Code waits for output from an agent before it ends the attempt: the call's own
/// `stallMs`, else `CLAUDE_ASYNC_AGENT_STALL_TIMEOUT_MS`, else the default (10 minutes, or the
/// host's `defaultMs`). A value that is not a positive whole number is no value (judgment).
#[must_use]
pub fn stall_window_ms(input: &StallWindowInput) -> f64 {
    is_positive_integer(input.stall_ms)
        .or_else(|| positive_integer(input.env.get(STALL_ENV).and_then(Option::as_deref)))
        .or_else(|| is_positive_integer(input.default_ms))
        .unwrap_or(DEFAULT_STALL_MS)
}

/// How long an agent has been silent.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SilenceInput {
    /// Milliseconds since the agent's last output.
    pub since_output_ms: f64,
    /// Of those, the time spent waiting on its own tool calls.
    pub tool_wait_ms: f64,
    /// Of those, the time spent waiting for a usage-limit reset.
    pub usage_limit_wait_ms: f64,
    /// The stall window.
    pub window_ms: f64,
}

/// "Time the agent spends waiting on its own tool calls or on a usage-limit reset doesn't count toward the stall window."
#[must_use]
pub fn has_stalled(input: SilenceInput) -> bool {
    let silent = input.since_output_ms - input.tool_wait_ms - input.usage_limit_wait_ms;
    silent >= input.window_ms
}

/// Where an `agent()` call sits in the script.
pub const CALL_SITES: [CallSite; 3] = [CallSite::Parallel, CallSite::Pipeline, CallSite::Awaited];

/// One of `CALL_SITES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallSite {
    /// Inside `parallel()`.
    Parallel,
    /// Inside `pipeline()`.
    Pipeline,
    /// Awaited directly.
    Awaited,
}

/// What happens to the run after an `agent()` call failed.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum AfterFailure {
    /// The run carries on with `null` in place of the agent's result.
    Continues {
        /// Always true.
        #[serde(rename = "runContinues")]
        run_continues: bool,
        /// Always `null`.
        result: (),
    },
    /// The run ends with the error.
    Ends {
        /// Always false.
        #[serde(rename = "runContinues")]
        run_continues: bool,
        /// The error.
        error: PluginIssue,
    },
}

const fn site_continues(site: CallSite) -> bool {
    match site {
        CallSite::Parallel | CallSite::Pipeline => true,
        CallSite::Awaited => false,
    }
}

/// Whether the run goes on after an `agent()` call failed: "Inside `parallel()` or `pipeline()`: the
/// run carries on with `null` in place of the agent's result. Awaited directly: the run ends with the
/// error."
#[must_use]
pub fn after_agent_failure(site: CallSite, failure: &AgentFailure) -> AfterFailure {
    if site_continues(site) {
        AfterFailure::Continues {
            run_continues: true,
            result: (),
        }
    } else {
        AfterFailure::Ends {
            run_continues: false,
            error: failure.issue.clone(),
        }
    }
}

crate::registry::exports! {
    "agents/workflow/attempts" {
        "ATTEMPT_ENDINGS" => ok || ATTEMPT_ENDINGS;
        "FAILURE_CAUSES" => ok || FAILURE_CAUSES;
        "attemptLabels" => ok |attempt: f64, previous: Option<AttemptEnding>| attempt_labels(attempt, previous);
        "afterAttempt" => ok |endings: Vec<AttemptEnding>| after_attempt(&endings);
        "canRestartOnRequest" => ok |attempt: f64| can_restart_on_request(attempt);
        "stallWindowMs" => ok |input: StallWindowInput| stall_window_ms(&input);
        "hasStalled" => ok |input: SilenceInput| has_stalled(input);
        "CALL_SITES" => ok || CALL_SITES;
        "afterAgentFailure" => ok |site: CallSite, failure: AgentFailure| after_agent_failure(site, &failure);
    }
}
