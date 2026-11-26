//! Port of `src/agents/workflow/usage-limit.ts`, switched.
//!
//! When a run hits your usage limit (workflows.md, "When a run hits your usage limit"): the run pauses
//! instead of failing the agent, when all of the conditions hold; when one does not, the affected agent
//! fails. Pure over facts the host holds.

use serde::{Deserialize, Serialize};

/// The kinds of session a run can be in.
pub const SESSION_KINDS: [SessionKind; 5] = [
    SessionKind::Interactive,
    SessionKind::NonInteractive,
    SessionKind::Background,
    SessionKind::RemoteControl,
    SessionKind::AgentTeamTeammate,
];

/// One of `SESSION_KINDS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionKind {
    /// A person at the terminal.
    Interactive,
    /// `claude -p` or the Agent SDK.
    NonInteractive,
    /// A background session.
    Background,
    /// Remote Control.
    RemoteControl,
    /// An agent team teammate.
    AgentTeamTeammate,
}

/// "The session is interactive": a run does not pause in `claude -p` or the Agent SDK, a background session, Remote Control or an agent team teammate.
const fn session_pauses(kind: SessionKind) -> bool {
    match kind {
        SessionKind::Interactive => true,
        SessionKind::NonInteractive
        | SessionKind::Background
        | SessionKind::RemoteControl
        | SessionKind::AgentTeamTeammate => false,
    }
}

/// The reasons the run does not wait.
pub const NO_WAIT_CAUSES: [NoWaitCause; 6] = [
    NoWaitCause::SessionNotInteractive,
    NoWaitCause::NoSubscription,
    NoWaitCause::AutoContinueOff,
    NoWaitCause::ResetTooFar,
    NoWaitCause::WaitedTwice,
    NoWaitCause::OldRelease,
];

/// One of `NO_WAIT_CAUSES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoWaitCause {
    /// The session is not interactive.
    SessionNotInteractive,
    /// Not signed in with a subscription.
    NoSubscription,
    /// `autoContinueAtUsageLimit` is off.
    AutoContinueOff,
    /// The limit resets in more than a day.
    ResetTooFar,
    /// The run already waited twice.
    WaitedTwice,
    /// The release has no wait.
    OldRelease,
}

/// A day, in milliseconds: "The limit resets within 24 hours".
const DAY_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

/// "The run hasn't already waited twice. When it hits the limit a third time, the agent fails."
const MAX_USAGE_WAITS: f64 = 2.0;

/// What the host knows when a run hits the limit.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLimitFacts {
    /// The kind of session.
    pub session: SessionKind,
    /// Signed in with a claude.ai subscription.
    pub subscription: bool,
    /// `autoContinueAtUsageLimit` is on.
    pub auto_continue: bool,
    /// Milliseconds until the limit resets. A weekly limit can reset further out.
    pub ms_until_reset: f64,
    /// How many times the run has already waited.
    pub waits_so_far: f64,
    /// The release has the wait (Claude Code v2.1.271 or later); "on earlier versions, the affected agents fail".
    pub release_has_wait: bool,
}

/// Whether the run waits, or the agent fails and why.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum UsageLimitDecision {
    /// The agents that hit the limit wait for the reset, and "no new agents start" while it lasts.
    Pauses {
        /// Always true.
        pauses: bool,
        /// Shown on the progress line and in the `/workflows` header.
        #[serde(rename = "msUntilReset")]
        ms_until_reset: f64,
    },
    /// The agent that hit the limit fails.
    Fails {
        /// Always false.
        pauses: bool,
        /// Always true.
        #[serde(rename = "failsAgent")]
        fails_agent: bool,
        /// Why it does not wait.
        causes: Vec<NoWaitCause>,
    },
}

fn condition(cause: NoWaitCause, facts: &UsageLimitFacts) -> bool {
    match cause {
        NoWaitCause::SessionNotInteractive => !session_pauses(facts.session),
        NoWaitCause::NoSubscription => !facts.subscription,
        NoWaitCause::AutoContinueOff => !facts.auto_continue,
        NoWaitCause::ResetTooFar => facts.ms_until_reset > DAY_MS,
        NoWaitCause::WaitedTwice => facts.waits_so_far >= MAX_USAGE_WAITS,
        NoWaitCause::OldRelease => !facts.release_has_wait,
    }
}

/// Whether the run waits for the reset or the agent that hit the limit fails. Asked again during a
/// wait with the current facts: "If you turn \[`autoContinueAtUsageLimit`\] off during a wait, the wait
/// ends and the waiting agents fail."
#[must_use]
pub fn usage_limit_decision(facts: &UsageLimitFacts) -> UsageLimitDecision {
    let causes: Vec<NoWaitCause> = NO_WAIT_CAUSES
        .into_iter()
        .filter(|cause| condition(*cause, facts))
        .collect();
    if causes.is_empty() {
        UsageLimitDecision::Pauses {
            pauses: true,
            ms_until_reset: facts.ms_until_reset,
        }
    } else {
        UsageLimitDecision::Fails {
            pauses: false,
            fails_agent: true,
            causes,
        }
    }
}

crate::registry::exports! {
    "agents/workflow/usage-limit" {
        "SESSION_KINDS" => ok || SESSION_KINDS;
        "NO_WAIT_CAUSES" => ok || NO_WAIT_CAUSES;
        "usageLimitDecision" => ok |facts: UsageLimitFacts| usage_limit_decision(&facts);
    }
}
