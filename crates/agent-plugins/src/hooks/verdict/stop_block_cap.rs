//! Port of `src/hooks/verdict/stop-block-cap.ts`, switched.
//!
//! The cap on a Stop hook that keeps blocking (env-vars.md, `CLAUDE_CODE_STOP_HOOK_BLOCK_CAP`):
//! "Maximum number of consecutive times a Stop or SubagentStop hook may block the turn from ending
//! before Claude Code overrides it and ends the turn anyway (default: 8). Set to `0` to disable the
//! cap." hooks.md ("Stop decision control") names the same loop protection, "the `stop_hook_active`
//! input and the 8-consecutive-continuation cap", for `additionalContext` that keeps the
//! conversation going too, so both kinds of continuation count.
//!
//! The count is the host's state: one counter per session (and per subagent for `SubagentStop`).
//! hooks.md ("Stop input"): "The count of consecutive continuations resets each time Claude calls a
//! tool", so the engine calls `reset` when a `PreToolUse`, `PostToolUse` or `PostToolUseFailure`
//! event is dispatched; a Stop that does not hold the turn back ends the run of blocks too. A host
//! that builds an engine per run injects a counter that outlives it (`HookEngineOptions.stopCap`)
//! and owns its lifetime: it calls `reset` when the session ends.
//!
//! Judgments (spec/judgments.md): a value that is not a whole number of at least 0 is the default;
//! the block that exceeds the cap is the one overridden (the first 8 count), and the override leaves
//! a warning issue, since the docs describe no message for it.
//!
//! The outcome is the host's whole value, so every member the cap does not change goes back as
//! given.

use std::sync::{Mutex, PoisonError};

use indexmap::IndexMap;
use serde::Deserialize;

use crate::format::env_number::whole_env;
use crate::format::issues::warning;
use crate::hooks::input::fact_read::text_fact_of;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, number_to_string};
use crate::ports::environment::Environment as _;
use crate::registry::{Handle, Port};

const DEFAULT_STOP_BLOCK_CAP: f64 = 8.0;
const VARIABLE: &str = "CLAUDE_CODE_STOP_HOOK_BLOCK_CAP";

/// The events the cap guards: the ones whose "block" keeps Claude working.
const GUARDED: [&str; 2] = ["Stop", "SubagentStop"];

/// The cap in force: `0` is "no cap".
async fn stop_block_cap(environment: Port) -> f64 {
    // J3.17: a whole number as every numeric variable reads it (`2e0`, `1_0`); `0` stays "no cap".
    whole_env(environment.get(VARIABLE).await.as_deref()).unwrap_or(DEFAULT_STOP_BLOCK_CAP)
}

/// The events after which a Stop starts counting again: "each time Claude calls a tool".
const TOOL_EVENTS: [&str; 3] = ["PreToolUse", "PostToolUse", "PostToolUseFailure"];

/// Whether this event is a tool call, which ends the run of consecutive blocks.
#[must_use]
pub fn is_tool_call(event: &str) -> bool {
    TOOL_EVENTS.contains(&event)
}

/// One counter per session; a subagent's stops are counted apart from the main agent's.
fn counter_key(session_id: &str, agent_id: Option<&str>) -> String {
    format!("{session_id}:{}", agent_id.unwrap_or_default())
}

/// The subagent a `SubagentStop`'s facts name (`agent_id`); a tool event's subagent is the session's.
fn agent_of(facts: &JsValue) -> Option<String> {
    text_fact_of(facts, "agent_id")
}

fn event_of(outcome: &JsObject) -> &str {
    outcome
        .get("event")
        .and_then(JsValue::as_str)
        .unwrap_or_default()
}

fn holds_turn(outcome: &JsObject) -> bool {
    outcome
        .get("block")
        .is_some_and(|block| !block.is_undefined())
        || outcome
            .get("effects")
            .and_then(|effects| effects.get("continuesTurn"))
            .and_then(JsValue::as_bool)
            == Some(true)
}

/// The outcome with the hold on the turn lifted: the turn ends.
fn overridden(mut outcome: JsObject, cap: f64) -> JsObject {
    let event = event_of(&outcome).to_owned();
    outcome.shift_remove("block");
    if let Some(JsValue::Object(effects)) = outcome.get_mut("effects") {
        effects.shift_remove("continuesTurn");
    }
    let message = format!(
        "A {event} hook held the turn back {} times in a row; Claude Code ends the turn anyway ({VARIABLE}).",
        number_to_string(cap)
    );
    let mut issues = outcome
        .get("issues")
        .and_then(JsValue::as_array)
        .cloned()
        .unwrap_or_default();
    issues.push(to_js_value(&warning(&format!("hooks.{event}"), &message)).unwrap_or_default());
    outcome.insert("proceed".to_owned(), JsValue::Bool(true));
    outcome.insert("issues".to_owned(), JsValue::Array(issues));
    outcome
}

/// Where a dispatch happened: the session and the event's facts.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapWhere {
    /// The session.
    pub session_id: String,
    /// The event's facts.
    #[serde(default)]
    pub facts: JsValue,
}

/// The counts of consecutive holds, by session and subagent.
#[derive(Debug, Default)]
pub struct StopBlockCap {
    counts: Mutex<IndexMap<String, f64>>,
}

impl StopBlockCap {
    fn with_counts<R>(&self, body: impl FnOnce(&mut IndexMap<String, f64>) -> R) -> R {
        body(&mut self.counts.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Counts this dispatch and overrides the hold once it exceeds the cap.
    pub async fn apply(&self, outcome: JsObject, at: CapWhere, environment: Port) -> JsObject {
        if !GUARDED.contains(&event_of(&outcome)) {
            return outcome;
        }
        let key = counter_key(&at.session_id, agent_of(&at.facts).as_deref());
        if !holds_turn(&outcome) {
            self.with_counts(|counts| counts.shift_remove(&key));
            return outcome;
        }
        let cap = stop_block_cap(environment).await;
        let count = self.with_counts(|counts| counts.get(&key).copied().unwrap_or(0.0)) + 1.0;
        if cap == 0.0 || count <= cap {
            self.with_counts(|counts| counts.insert(key, count));
            return outcome;
        }
        self.with_counts(|counts| counts.shift_remove(&key));
        overridden(outcome, cap)
    }

    /// Starts the count again for a session (and a subagent): Claude called a tool, or the session ended.
    pub fn reset(&self, session_id: &str, agent_id: Option<&str>) {
        self.with_counts(|counts| counts.shift_remove(&counter_key(session_id, agent_id)));
    }
}

/// A counter of its own.
#[must_use]
pub fn create_stop_block_cap() -> StopBlockCap {
    StopBlockCap::default()
}

crate::registry::methods! {
    StopBlockCap {
        "apply" => sync async ok |this, outcome: JsObject, at: CapWhere, environment: Port|
            this.apply(outcome, at, environment).await;
        "reset" => ok |this, session_id: String, agent_id: Option<String>|
            this.reset(&session_id, agent_id.as_deref());
    }
}

crate::registry::exports! {
    "hooks/verdict/stop-block-cap" {
        "createStopBlockCap" => ok || Handle::own(create_stop_block_cap());
    }
}
