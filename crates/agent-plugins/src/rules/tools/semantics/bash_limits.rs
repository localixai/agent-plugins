//! Port of `src/rules/tools/semantics/bash-limits.ts`, switched.
//!
//! How long a Bash or PowerShell command runs, in the foreground and in the background
//! (tools-reference.md, "Timeout and output limits", "Time limit for background commands").

use serde::Deserialize;

use super::env_number::positive_integer;
use crate::js::{JsValue, math_max, math_min};
use crate::ports::environment::Environment;
use crate::registry::Port;

/// tools-reference.md, "Timeout and output limits" and env-vars.md: milliseconds.
const DEFAULT_TIMEOUT_MS: f64 = 120_000.0;
const MAX_TIMEOUT_MS: f64 = 600_000.0;
const BACKGROUND_DEFAULT_MS: f64 = 1_800_000.0;
const BACKGROUND_PRINT_DEFAULT_MS: f64 = 600_000.0;
const BACKGROUND_MAX_MS: f64 = 7_200_000.0;

/// `BASH_DEFAULT_TIMEOUT_MS` and `BASH_MAX_TIMEOUT_MS`, each when a positive integer.
struct Configured {
    default_ms: Option<f64>,
    max_ms: Option<f64>,
}

async fn configured(env: &dyn Environment) -> Configured {
    Configured {
        default_ms: positive_integer(env, "BASH_DEFAULT_TIMEOUT_MS").await,
        max_ms: positive_integer(env, "BASH_MAX_TIMEOUT_MS").await,
    }
}

/// What a foreground command may run for: the default, and the ceiling over whatever Claude asks.
async fn foreground_limits(env: &dyn Environment) -> (f64, f64) {
    let Configured { default_ms, max_ms } = configured(env).await;
    let fallback = default_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    (
        fallback,
        math_max(max_ms.unwrap_or(MAX_TIMEOUT_MS), fallback),
    )
}

fn is_positive_number(value: &JsValue) -> Option<f64> {
    value
        .as_f64()
        .filter(|number| number.is_finite() && *number > 0.0)
}

/// How long a foreground Bash or PowerShell command runs (tools-reference.md, "Timeout and output
/// limits"; hooks.md: "Values above the maximum are reduced to the maximum rather than rejected").
/// `BASH_DEFAULT_TIMEOUT_MS` is the default when Claude passes no timeout, two minutes out of the
/// box; `BASH_MAX_TIMEOUT_MS` sets the ceiling, ten minutes out of the box, and the effective
/// ceiling is the larger of the two. A timeout that is not a positive number counts as none, as
/// does a variable that is not a positive integer (spec/judgments.md).
pub async fn effective_bash_timeout(requested: &JsValue, env: &dyn Environment) -> f64 {
    let (fallback, ceiling) = foreground_limits(env).await;
    is_positive_number(requested).map_or(fallback, |requested| math_min(requested, ceiling))
}

/// What the session is, as far as the background time limit goes.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundSession {
    /// A run with `-p`, an Agent SDK application, a CI job or a cloud session; a local session has
    /// no limit.
    pub unattended: bool,
    /// A `-p` run whose prompt is text, not `--input-format stream-json`: its defaults are 10
    /// minutes.
    pub print_with_text_prompt: bool,
}

/// A background command whose time limit is asked.
pub struct BackgroundRequest<'a> {
    /// The session.
    pub session: BackgroundSession,
    /// The environment.
    pub env: &'a dyn Environment,
    /// The `timeout` Claude passed with `run_in_background`.
    pub timeout: JsValue,
    /// A foreground command that reached its timeout and moved to the background.
    pub moved: Option<bool>,
}

/// How long a background command may run, or `None` for none (tools-reference.md, "Time limit
/// for background commands"). A command Claude starts in the background gets 30 minutes, or its
/// `timeout` up to 2 hours; one that moves from the foreground gets 30 minutes from the move.
/// `BASH_DEFAULT_TIMEOUT_MS` above the default replaces it (a lower value never shortens it), and
/// an effective ceiling above 2 hours raises the maximum.
pub async fn background_time_limit(request: BackgroundRequest<'_>) -> Option<f64> {
    let BackgroundRequest {
        session,
        env,
        timeout,
        moved,
    } = request;
    if !session.unattended {
        return None;
    }
    let base = if session.print_with_text_prompt {
        BACKGROUND_PRINT_DEFAULT_MS
    } else {
        BACKGROUND_DEFAULT_MS
    };
    let Configured { default_ms, max_ms } = configured(env).await;
    let limit = math_max(base, default_ms.unwrap_or(0.0));
    let maximum = math_max(
        math_max(BACKGROUND_MAX_MS, default_ms.unwrap_or(0.0)),
        max_ms.unwrap_or(0.0),
    );
    match is_positive_number(&timeout) {
        Some(timeout) if moved != Some(true) => Some(math_min(timeout, maximum)),
        _ => Some(limit),
    }
}

/// `BackgroundRequest` as it crosses the boundary: its `env` is the host's port.
#[derive(Deserialize)]
struct BackgroundRequestData {
    session: BackgroundSession,
    env: Port,
    #[serde(default)]
    timeout: JsValue,
    #[serde(default)]
    moved: Option<bool>,
}

async fn background_time_limit_of(request: BackgroundRequestData) -> Option<f64> {
    background_time_limit(BackgroundRequest {
        session: request.session,
        env: &request.env,
        timeout: request.timeout,
        moved: request.moved,
    })
    .await
}

crate::registry::exports! {
    "rules/tools/semantics/bash-limits" {
        "effectiveBashTimeout" => async ok |requested: JsValue, env: Port| effective_bash_timeout(&requested, &env).await;
        "backgroundTimeLimit" => async ok |request: BackgroundRequestData| background_time_limit_of(request).await;
    }
}
