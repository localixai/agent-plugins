//! Port of `src/rules/tools/semantics/web-search-limit.ts`, switched.
//!
//! WebSearch's session limit and how many calls it still allows (tools-reference.md, "Session
//! search limit").

use serde::{Deserialize, Serialize};

use super::env_number::{positive_integer, whole_number};
use crate::js::math_max;
use crate::ports::environment::Environment;
use crate::registry::Port;

/// tools-reference.md, "Session search limit": "a limit of 200 WebSearch calls".
const DEFAULT_CAP: f64 = 200.0;
/// "refills at about 100 calls per hour" in an interactive terminal session.
const INTERACTIVE_REFILLS_PER_HOUR: f64 = 100.0;
const HOUR_MS: f64 = 3_600_000.0;

/// The session WebSearch runs in.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct WebSearchSession {
    /// An interactive terminal session; `false` for `-p` and the Agent SDK.
    pub interactive: bool,
}

/// The limits of WebSearch.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchLimits {
    /// The most calls the count may reach.
    pub cap: f64,
    /// Calls given back per hour.
    pub refill_per_hour: f64,
    /// "In non-interactive mode with `-p` and in the Agent SDK, the limit applies per turn, not
    /// per session."
    pub per_turn: bool,
}

/// The limits of WebSearch (tools-reference.md, "Session search limit"; env-vars.md).
/// `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` "accepts a positive whole number"; anything else is
/// ignored, so the cap "can be raised but not turned off".
/// `CLAUDE_CODE_WEB_SEARCH_REFILLS_PER_HOUR` takes plain digits (`0` included); the default is 100
/// in an interactive session and 0, no refill, in a non-interactive one. A number below 200 lowers
/// the cap (spec/judgments.md).
pub async fn web_search_limits(
    env: &dyn Environment,
    session: WebSearchSession,
) -> WebSearchLimits {
    let cap = positive_integer(env, "CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION")
        .await
        .unwrap_or(DEFAULT_CAP);
    let refill = whole_number(env, "CLAUDE_CODE_WEB_SEARCH_REFILLS_PER_HOUR").await;
    WebSearchLimits {
        cap,
        refill_per_hour: refill.unwrap_or(if session.interactive {
            INTERACTIVE_REFILLS_PER_HOUR
        } else {
            0.0
        }),
        per_turn: !session.interactive,
    }
}

/// The calls counted so far.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchUsage {
    /// Calls counted so far in the session, or in the turn when the limits are per turn.
    pub used: f64,
    /// Milliseconds since the count started.
    pub elapsed_ms: f64,
}

/// How many more WebSearch calls are allowed. The count refills at the hourly rate, continuously
/// and rounded down, and never gives back more than was used (spec/judgments.md: the docs say
/// "about 100 calls per hour" and no more). At zero a call "did nothing" and Claude is told to
/// carry on.
#[must_use]
pub fn web_search_allowance(limits: WebSearchLimits, usage: WebSearchUsage) -> f64 {
    let refilled = ((limits.refill_per_hour * usage.elapsed_ms) / HOUR_MS).floor();
    let counted = math_max(usage.used - refilled, 0.0);
    math_max(limits.cap - counted, 0.0)
}

crate::registry::exports! {
    "rules/tools/semantics/web-search-limit" {
        "webSearchLimits" => async ok |env: Port, session: WebSearchSession| web_search_limits(&env, session).await;
        "webSearchAllowance" => ok |limits: WebSearchLimits, usage: WebSearchUsage| web_search_allowance(limits, usage);
    }
}
