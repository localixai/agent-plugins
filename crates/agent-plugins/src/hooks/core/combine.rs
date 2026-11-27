//! Port of `src/hooks/core/combine.ts`, switched.
//!
//! One occurrence's outcome with its core side added (Agent Plugins Extended §18.1.2, "Combining
//! results"): Claude Code's outcome is what the engine made of its handlers, unchanged in what it
//! says of them (principle 1); the core results join it by the event's rule. A failed core handler
//! contributes nothing but its notice (J18.4). Every Claude Code handler precedes every core one in
//! handler order, and core handlers follow the order their packages loaded (J18.3).
//!
//! The outcome is the host's whole value (its background promises among it), so every member the
//! combination does not change goes back as given, in its place.

use serde::Deserialize;

use super::types::{CoreDecision, CoreOccurrence, CoreResult, CoreRun};
use crate::format::core::hooks::CoreHookEvent;
use crate::hooks::verdict::blocked_prompt::with_original_prompt;
use crate::hooks::verdict::notices::{hook_error, hook_name};
use crate::hooks::verdict::route::{BlockText, routed_block};
use crate::hooks::verdict::types::BlockVia;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};

/// The core side of one occurrence.
#[derive(Clone, Debug, Deserialize)]
pub struct CoreSide {
    /// The occurrence.
    pub occurrence: CoreOccurrence,
    /// Each core handler's run.
    pub runs: Vec<CoreRun>,
    /// Every successful `additional_context`, already capped and with its reminder tags escaped.
    pub contexts: Vec<String>,
    /// The issues of the core side.
    pub issues: Vec<JsValue>,
}

type Outcome = JsObject;
type Combine = fn(Outcome, &CoreSide, &JsValue) -> Outcome;

struct Found {
    handler_id: String,
    reason: String,
}

fn decisions_of(side: &CoreSide, decision: CoreDecision) -> Vec<Found> {
    side.runs
        .iter()
        .filter_map(|run| match &run.result {
            Some(CoreResult::Decision {
                decision: given,
                reason,
            }) if *given == decision => Some(Found {
                handler_id: run.handler_id.clone(),
                reason: reason.clone().unwrap_or_default(),
            }),
            _ => None,
        })
        .collect()
}

/// J18.22: the reasons in handler order, an absent or empty one left out, one newline between.
fn joined_reasons(found: &[Found]) -> String {
    found
        .iter()
        .map(|one| one.reason.as_str())
        .filter(|reason| !reason.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strings_of(outcome: &Outcome, key: &str) -> Vec<JsValue> {
    outcome
        .get(key)
        .and_then(JsValue::as_array)
        .cloned()
        .unwrap_or_default()
}

fn effects_of(outcome: &Outcome) -> JsObject {
    outcome
        .get("effects")
        .and_then(JsValue::as_object)
        .cloned()
        .unwrap_or_default()
}

fn set(mut outcome: Outcome, key: &str, value: JsValue) -> Outcome {
    outcome.insert(key.to_owned(), value);
    outcome
}

fn with_contexts(outcome: Outcome, side: &CoreSide, _facts: &JsValue) -> Outcome {
    let mut contexts = strings_of(&outcome, "additionalContext");
    contexts.extend(
        side.contexts
            .iter()
            .map(|context| JsValue::from(context.as_str())),
    );
    set(outcome, "additionalContext", JsValue::Array(contexts))
}

/// A block of `event` by `by`, routed as the event routes a decision.
fn decision_block(event: &str, reason: &str, by: &str) -> JsValue {
    let routed = routed_block(
        event,
        BlockVia::Decision,
        BlockText {
            reason: reason.to_owned(),
            message: reason.to_owned(),
        },
        None,
    );
    let mut block = match to_js_value(&routed) {
        Ok(JsValue::Object(members)) => members,
        _ => JsObject::new(),
    };
    block.insert("by".to_owned(), JsValue::from(by));
    JsValue::Object(block)
}

fn proceeds(outcome: &Outcome) -> bool {
    outcome.get("proceed").and_then(JsValue::as_bool) == Some(true)
}

/// `UserPromptSubmit`: blocked when either side blocks; a blocked prompt delivers no context from
/// either side (core §7.3.5). A Claude Code block keeps its own message; else the core reasons are
/// the message, ending with the prompt (Claude Code's `suppressOriginalPrompt` belongs to a
/// blocking Claude Code handler only, §7.5.8).
fn user_prompt_submit(outcome: Outcome, side: &CoreSide, facts: &JsValue) -> Outcome {
    if !proceeds(&outcome) {
        return set(outcome, "additionalContext", JsValue::Array(vec![]));
    }
    let denials = decisions_of(side, CoreDecision::Deny);
    let Some(first) = denials.first() else {
        return with_contexts(outcome, side, facts);
    };
    let reason = joined_reasons(&denials);
    let given = effects_of(&outcome);
    let mut effects = given.clone();
    effects.shift_remove("suppressOriginalPrompt");
    let blocked = set(outcome, "proceed", JsValue::Bool(false));
    let blocked = set(
        blocked,
        "block",
        decision_block("UserPromptSubmit", &reason, &first.handler_id),
    );
    let blocked = set(blocked, "additionalContext", JsValue::Array(vec![]));
    let blocked = set(blocked, "effects", JsValue::Object(effects));
    set(
        with_original_prompt(blocked, facts),
        "effects",
        JsValue::Object(given),
    )
}

/// `PreToolUse`: `deny` > `defer` > `ask` > `allow` > none across both sides. A Claude Code
/// denial (an exit 2 included) keeps its reason; a core denial denies with the core reasons and
/// runs nothing, so no rewrite applies; a core `allow` is an `allow` with no `updatedInput`, and
/// wins only over no decision at all.
fn pre_tool_use(outcome: Outcome, side: &CoreSide, _facts: &JsValue) -> Outcome {
    let effects = effects_of(&outcome);
    let own = if outcome.get("block").is_none_or(JsValue::is_undefined) {
        effects
            .get("permissionDecision")
            .and_then(JsValue::as_str)
            .map(str::to_owned)
    } else {
        Some("deny".to_owned())
    };
    if own.as_deref() == Some("deny") {
        return outcome;
    }
    let denials = decisions_of(side, CoreDecision::Deny);
    if let Some(first) = denials.first() {
        let reason = joined_reasons(&denials);
        let mut effects = effects;
        effects.shift_remove("updatedInput");
        effects.insert("permissionDecision".to_owned(), JsValue::from("deny"));
        effects.insert(
            "permissionDecisionReason".to_owned(),
            JsValue::from(reason.as_str()),
        );
        let denied = set(outcome, "proceed", JsValue::Bool(false));
        let denied = set(
            denied,
            "block",
            decision_block("PreToolUse", &reason, &first.handler_id),
        );
        return set(denied, "effects", JsValue::Object(effects));
    }
    let allows = !decisions_of(side, CoreDecision::Allow).is_empty();
    if allows && own.is_none() {
        let mut effects = effects;
        effects.insert("permissionDecision".to_owned(), JsValue::from("allow"));
        return set(outcome, "effects", JsValue::Object(effects));
    }
    outcome
}

fn appended(first: &str, more: &str) -> String {
    if first.is_empty() || more.is_empty() {
        format!("{first}{more}")
    } else {
        format!("{first}\n{more}")
    }
}

fn text_member(block: &JsObject, key: &str) -> String {
    block
        .get(key)
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// `Stop`: one continuation when either side asks, unless a Claude Code handler returned
/// `continue: false` (J18.9). It carries every reason: a Claude Code block's own, then the core
/// reasons. The cap counts it once, after this (the session applies it to the combined outcome).
fn stop(outcome: Outcome, side: &CoreSide, _facts: &JsValue) -> Outcome {
    let continues = decisions_of(side, CoreDecision::Continue);
    let goes_on = outcome.get("continue").and_then(JsValue::as_bool) == Some(true);
    let Some(first) = continues.first().filter(|_| goes_on) else {
        return outcome;
    };
    let reason = joined_reasons(&continues);
    if let Some(JsValue::Object(block)) = outcome.get("block") {
        let mut block = block.clone();
        let held = appended(&text_member(&block, "reason"), &reason);
        let message = appended(&text_member(&block, "message"), &reason);
        block.insert("reason".to_owned(), JsValue::from(held));
        block.insert("message".to_owned(), JsValue::from(message));
        return set(outcome, "block", JsValue::Object(block));
    }
    let held = set(outcome, "proceed", JsValue::Bool(false));
    set(
        held,
        "block",
        decision_block("Stop", &reason, &first.handler_id),
    )
}

const fn combine_of(event: CoreHookEvent) -> Combine {
    match event {
        CoreHookEvent::SessionStart | CoreHookEvent::PostToolUse => with_contexts,
        CoreHookEvent::UserPromptSubmit => user_prompt_submit,
        CoreHookEvent::PreToolUse => pre_tool_use,
        CoreHookEvent::Stop => stop,
    }
}

/// A core run as the outcome's run list records it.
fn run_of(run: &CoreRun) -> JsValue {
    let timed_out = run.timed_out == Some(true);
    let mut members = JsObject::from_iter([
        (
            "handlerId".to_owned(),
            JsValue::from(run.handler_id.as_str()),
        ),
        ("handler".to_owned(), JsValue::from(run.label.as_str())),
        ("plugin".to_owned(), JsValue::from(run.plugin_id.as_str())),
        (
            "ending".to_owned(),
            JsValue::from(if timed_out { "timed-out" } else { "finished" }),
        ),
    ]);
    if let Some(exit_code) = run.exit_code {
        members.insert("exitCode".to_owned(), JsValue::Number(exit_code));
    }
    if timed_out {
        members.insert("timeoutMs".to_owned(), JsValue::Number(run.timeout_ms));
    }
    let status = match (&run.failure, timed_out) {
        (None, _) => "success",
        (Some(_), true) => "timed-out",
        (Some(_), false) => "non-blocking-error",
    };
    members.insert("status".to_owned(), JsValue::from(status));
    JsValue::Object(members)
}

/// J18.4: a failed core handler is reported as a non-blocking hook error, `<hook name> hook
/// error`.
fn notices_of(side: &CoreSide) -> Vec<JsValue> {
    side.runs
        .iter()
        .filter_map(|run| {
            let failure = run.failure.as_ref()?;
            let name = hook_name(
                side.occurrence.event.as_str(),
                side.occurrence.tool_name.as_deref(),
            );
            Some(JsValue::Object(JsObject::from_iter([
                (
                    "handlerId".to_owned(),
                    JsValue::from(run.handler_id.as_str()),
                ),
                (
                    "message".to_owned(),
                    JsValue::from(hook_error(&name, &run.label, failure)),
                ),
            ])))
        })
        .collect()
}

fn joined(outcome: &Outcome, key: &str, more: Vec<JsValue>) -> JsValue {
    let mut all = strings_of(outcome, key);
    all.extend(more);
    JsValue::Array(all)
}

/// The outcome of the occurrence with the core side combined into it.
#[must_use]
pub fn combine_core(outcome: &Outcome, side: &CoreSide, facts: &JsValue) -> Outcome {
    let combined = combine_of(side.occurrence.event)(outcome.clone(), side, facts);
    let mut result = set(
        outcome.clone(),
        "proceed",
        combined.get("proceed").cloned().unwrap_or_default(),
    );
    if let Some(block) = combined.get("block").filter(|block| !block.is_undefined()) {
        result.insert("block".to_owned(), block.clone());
    }
    for key in ["additionalContext", "effects"] {
        result.insert(
            key.to_owned(),
            combined.get(key).cloned().unwrap_or_default(),
        );
    }
    let notices = joined(outcome, "notices", notices_of(side));
    let runs = joined(outcome, "runs", side.runs.iter().map(run_of).collect());
    let issues = joined(outcome, "issues", side.issues.clone());
    result.insert("notices".to_owned(), notices);
    result.insert("runs".to_owned(), runs);
    result.insert("issues".to_owned(), issues);
    result
}

crate::registry::exports! {
    "hooks/core/combine" {
        "combineCore" => ok |outcome: JsObject, side: CoreSide, facts: JsValue|
            combine_core(&outcome, &side, &facts);
    }
}
