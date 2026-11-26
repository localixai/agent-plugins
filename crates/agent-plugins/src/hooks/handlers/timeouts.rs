//! Port of `src/hooks/handlers/timeouts.ts`, switched.
//!
//! How long a handler may run (hooks.md, "Common fields", `timeout`): the kit computes the limit
//! and hands it to the port, which enforces it and reports `timedOut`.

use serde::{Deserialize, Serialize};

use crate::format::env_number::positive_whole_env;
use crate::hooks::source_kind::source_policy;
use crate::js::{JsObject, JsValue, is_truthy};
use crate::ports::environment::Environment;
use crate::registry::Port;

/// "Defaults: 600 for `command`, `http`, and `mcp_tool`; 30 for `prompt`; 60 for `agent`".
fn default_seconds(handler_type: &str) -> f64 {
    match handler_type {
        "prompt" => 30.0,
        "agent" => 60.0,
        _ => 600.0,
    }
}

/// The types whose default an event lowers: "the `command`, `http`, and `mcp_tool` default".
fn is_lowered_type(handler_type: &str) -> bool {
    matches!(handler_type, "command" | "http" | "mcp_tool")
}

/// "Claude Code kills a command hook that is still running" when the 10 seconds after `Ctrl+Enter`
/// are up (hooks.md, "PostToolUse after an interrupt"): only a command hook. Judgment
/// (spec/judgments.md): the other types keep their own `timeout`.
fn interrupt_kills(handler_type: &str) -> bool {
    handler_type == "command"
}

/// "30 on UserPromptSubmit, PreModelSwitch, and PostModelSwitch, and to 10 on MessageDisplay".
fn event_default_seconds(event: &str) -> Option<f64> {
    match event {
        "UserPromptSubmit" | "PreModelSwitch" | "PostModelSwitch" => Some(30.0),
        "MessageDisplay" => Some(10.0),
        _ => None,
    }
}

const SESSION_END_DEFAULT_MS: f64 = 1500.0;
const SESSION_END_MAX_BUDGET_MS: f64 = 60_000.0;
/// "Set this environment variable in milliseconds to override the budget explicitly".
const SESSION_END_OVERRIDE: &str = "CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS";

/// The limits `SessionEnd` hooks share: one budget for the event, and the default of each hook.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEndLimits {
    /// What the hooks share.
    pub budget_ms: f64,
    /// What a hook with no `timeout` gets.
    pub default_ms: f64,
}

/// What an event adds to the timeouts of its handlers; computed once per dispatch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutPlan {
    /// The limits `SessionEnd` hooks share.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_end: Option<SessionEndLimits>,
    /// An interrupt caps every hook that has a limit (hooks.md, "PostToolUse after an interrupt").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interrupt_cap_ms: Option<f64>,
}

/// The part of a registered handler the limits read.
#[derive(Clone, Debug, Deserialize)]
pub struct TimedEntry {
    /// The event it fires on.
    pub event: String,
    /// The official handler object.
    pub handler: JsObject,
    /// Where it was declared.
    pub source: TimedSource,
}

/// The kind of a declaration site.
#[derive(Clone, Debug, Deserialize)]
pub struct TimedSource {
    /// `settings`, `plugin`, `skill`, `agent` or `session`.
    pub kind: String,
}

fn seconds(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

/// The handler's `timeout` field, in seconds, when it is a positive number.
fn own_seconds(handler: &JsObject) -> Option<f64> {
    seconds(handler.get("timeout").and_then(JsValue::as_f64))
}

fn to_ms(value: f64) -> f64 {
    (value * 1000.0).round()
}

fn handler_type(entry: &TimedEntry) -> &str {
    entry
        .handler
        .get("type")
        .and_then(JsValue::as_str)
        .unwrap_or_default()
}

/// A positive whole number of milliseconds (`2e3`, `64_000`: J3.17), or `None` for text that is not
/// one.
async fn override_ms(environment: &Port) -> Option<f64> {
    positive_whole_env(environment.get(SESSION_END_OVERRIDE).await.as_deref())
}

/// "SessionEnd hooks share a 1.5-second budget; if your settings set a longer per-hook `timeout`,
/// Claude Code raises the budget to match, up to 60 seconds." "Timeouts set on plugin-provided hooks
/// don't raise the budget." The environment variable sets the budget and the default of a hook
/// with no `timeout` of its own; a hook that has one keeps it.
///
/// Judgment (spec/judgments.md): only settings-file hooks raise the budget (the docs say "your
/// settings files"); a hook never runs longer than the budget, since the budget is shared.
async fn session_end_limits(selected: &[TimedEntry], environment: &Port) -> SessionEndLimits {
    if let Some(over) = override_ms(environment).await {
        return SessionEndLimits {
            budget_ms: over,
            default_ms: over,
        };
    }
    let policies = source_policy();
    let longest = selected
        .iter()
        .filter(|entry| {
            policies
                .get(entry.source.kind.as_str())
                .is_some_and(|policy| policy.raises_session_end_budget)
        })
        .map(|entry| to_ms(own_seconds(&entry.handler).unwrap_or(0.0)))
        .fold(0.0, f64::max);
    SessionEndLimits {
        budget_ms: SESSION_END_DEFAULT_MS
            .max(longest)
            .min(SESSION_END_MAX_BUDGET_MS),
        default_ms: SESSION_END_DEFAULT_MS,
    }
}

/// The plan of one dispatch: `selected` are the handlers of `event` that will run.
pub async fn plan_timeouts(
    event: &str,
    selected: &[TimedEntry],
    environment: &Port,
    interrupt_cap_ms: Option<f64>,
) -> TimeoutPlan {
    let session_end = if event == "SessionEnd" {
        Some(session_end_limits(selected, environment).await)
    } else {
        None
    };
    TimeoutPlan {
        session_end,
        interrupt_cap_ms,
    }
}

/// "Claude Code doesn't enforce it on a command hook you run with `async: true`", but "still
/// enforces `timeout` on a hook you run with `asyncRewake`".
fn unbounded(handler: &JsObject) -> bool {
    let flag = |key: &str| handler.get(key).is_some_and(is_truthy);
    flag("async") && !flag("asyncRewake")
}

fn default_ms(entry: &TimedEntry, plan: &TimeoutPlan) -> f64 {
    if let Some(session_end) = plan.session_end {
        return session_end.default_ms;
    }
    let kind = handler_type(entry);
    let lowered = if is_lowered_type(kind) {
        event_default_seconds(&entry.event)
    } else {
        None
    };
    to_ms(lowered.unwrap_or_else(|| default_seconds(kind)))
}

/// The limit in milliseconds the port enforces on this handler, or `None` for none (an `async`
/// command hook). Judgment: a `timeout` that is not a positive number counts as unset.
#[must_use]
pub fn handler_timeout_ms(entry: &TimedEntry, plan: &TimeoutPlan) -> Option<f64> {
    if unbounded(&entry.handler) {
        return None;
    }
    let limit = own_seconds(&entry.handler).map_or_else(|| default_ms(entry, plan), to_ms);
    let budget = plan
        .session_end
        .map_or(limit, |session_end| limit.min(session_end.budget_ms));
    // "kills a command hook that is still running, whatever its `timeout`": the other types keep
    // theirs.
    match plan.interrupt_cap_ms {
        Some(cap) if interrupt_kills(handler_type(entry)) => Some(budget.min(cap)),
        _ => Some(budget),
    }
}

crate::registry::exports! {
    "hooks/handlers/timeouts" {
        "planTimeouts" => async ok |event: String, selected: Vec<TimedEntry>, environment: Port, interrupt_cap_ms: Option<f64>| plan_timeouts(&event, &selected, &environment, interrupt_cap_ms).await;
        "handlerTimeoutMs" => ok |entry: TimedEntry, plan: TimeoutPlan| handler_timeout_ms(&entry, &plan);
    }
}
