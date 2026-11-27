//! Port of `src/host/session-core-dispatch.ts`, switched.
//!
//! One occurrence for both forms (Agent Plugins Extended §18.1.2, "When each side runs"): with no
//! core handler on it the dispatch is Claude Code's alone, exactly as without core packages
//! (principle 1). `PreToolUse` runs Claude Code's handlers first and the core's on the input the call
//! would now run with, a denial included (J18.1); every other shared event runs both sides together
//! and waits for both (J18.2). On `Stop` the cap counts the combined continuation once (J18.9).
//!
//! The Claude Code side is the host's function (`claude(capped)`, the session's own dispatch over
//! its engine); the core side, the combination and the cap are the core's.

use serde::Deserialize;

use super::session_core_hooks::{CoreHandlerUse, run_core_side, selected_core_handlers};
use crate::format::core::hooks::CoreHookEvent;
use crate::hooks::core::combine::combine_core;
use crate::hooks::core::input::{OccurrenceContext, core_occurrence, needs_turn_origin};
use crate::hooks::core::types::{CoreCallFacts, CoreOccurrence};
use crate::hooks::verdict::stop_block_cap::{CapWhere, StopBlockCap};
use crate::js::{JsObject, JsValue, promise_all_pair};
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Callback, Own};

/// Where the occurrence happens.
#[derive(Debug, Deserialize)]
pub struct DispatchContext {
    /// The working directory.
    pub cwd: String,
    /// The hooks fire in a subagent.
    pub subagent: bool,
}

/// One dispatch of a session, both forms (`CoreDispatch`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreDispatch {
    /// The event.
    pub event: String,
    /// Its facts.
    pub facts: JsValue,
    /// What the core side reads.
    #[serde(rename = "use")]
    pub use_: CoreHandlerUse,
    /// Where it happens.
    pub context: DispatchContext,
    /// What the host knows of the call.
    #[serde(default)]
    pub core: Option<CoreCallFacts>,
    /// A limit to some sources of hooks.
    #[serde(default)]
    pub sources_only: Option<Vec<String>>,
    /// The session's Stop cap.
    pub stop_cap: Own<StopBlockCap>,
    /// Aborts the dispatch.
    pub signal: Signal,
}

/// The decisions under which no `updatedInput` applies (§7.5.11; the core's `hooks/output/merge_gates.rs`).
const KEEPS_INPUT: [Option<&str>; 3] = [None, Some("allow"), Some("ask")];

/// The input the call would run with after Claude Code's handlers (J18.1).
fn effective_facts(outcome: &JsObject, facts: &JsValue) -> JsValue {
    let effects = outcome.get("effects");
    let member = |name: &str| {
        effects
            .and_then(|it| it.get(name))
            .filter(|it| !it.is_undefined())
    };
    let decision = member("permissionDecision").and_then(JsValue::as_str);
    let blocked = outcome
        .get("block")
        .is_some_and(|block| !block.is_undefined());
    let applies = !blocked && KEEPS_INPUT.contains(&decision);
    match (member("updatedInput"), facts) {
        (Some(updated), JsValue::Object(members)) if applies => {
            let mut members = members.clone();
            members.insert("tool_input".to_owned(), updated.clone());
            JsValue::Object(members)
        }
        _ => facts.clone(),
    }
}

fn occurrence_of(request: &CoreDispatch, facts: &JsValue) -> Option<CoreOccurrence> {
    let context = OccurrenceContext {
        session_id: request.use_.session.session_id.clone(),
        cwd: request.context.cwd.clone(),
        subagent: request.context.subagent,
        facts: request.core.clone().unwrap_or_default(),
    };
    core_occurrence(&request.event, facts, &context)
}

/// J18.7: a `system` turn holds peer and channel messages, which fire core `UserPromptSubmit`, and
/// task notifications and automatic continuations, which do not. Only the host knows which, so with
/// a core handler to run the dispatch refuses one without `turnOrigin` rather than guess.
fn require_turn_origin(request: &CoreDispatch) -> Result<(), String> {
    let core = request.core.clone().unwrap_or_default();
    if !needs_turn_origin(&request.event, &request.facts, &core) {
        return Ok(());
    }
    if selected_core_handlers(&request.use_, CoreHookEvent::UserPromptSubmit).is_empty() {
        return Ok(());
    }
    Err("dispatch UserPromptSubmit: a turn whose source is \"system\" needs SessionDispatchOptions.core.turnOrigin (\"external\" or \"client\") when a core package handles UserPromptSubmit (Agent Plugins Extended §18.1.2, J18.7)".to_owned())
}

/// Whether core handlers take part: a dispatch limited to sources without plugins has none.
fn first_occurrence(request: &CoreDispatch) -> Result<Option<CoreOccurrence>, String> {
    if let Some(only) = &request.sources_only
        && !only.iter().any(|kind| kind == "plugin")
    {
        return Ok(None);
    }
    require_turn_origin(request)?;
    let Some(occurrence) = occurrence_of(request, &request.facts) else {
        return Ok(None);
    };
    let none = selected_core_handlers(&request.use_, occurrence.event).is_empty();
    Ok((!none).then_some(occurrence))
}

/// The dispatch of both forms; `claude(capped)` runs the Claude Code side.
///
/// # Errors
///
/// A `system` turn without `turnOrigin` when a core package handles `UserPromptSubmit` (J18.7).
pub async fn dispatch_both_forms(
    request: CoreDispatch,
    claude: Callback,
) -> Result<JsObject, String> {
    let Some(first) = first_occurrence(&request)? else {
        return Ok(claude.call::<JsObject>(vec![JsValue::Bool(true)]).await);
    };
    let CoreDispatch {
        facts,
        signal,
        use_,
        ..
    } = &request;
    if first.event == CoreHookEvent::PreToolUse {
        let outcome: JsObject = claude.call(vec![JsValue::Bool(true)]).await;
        let occurrence =
            occurrence_of(&request, &effective_facts(&outcome, facts)).unwrap_or(first);
        return Ok(combine_core(
            &outcome,
            &run_core_side(use_, occurrence, signal).await,
            facts,
        ));
    }
    let stop = first.event == CoreHookEvent::Stop;
    let claude_side: BoxFuture<'_, JsObject> = Box::pin(claude.call(vec![JsValue::Bool(!stop)]));
    let core_side = Box::pin(run_core_side(use_, first, signal));
    let (outcome, side) = promise_all_pair(claude_side, core_side).await;
    let combined = combine_core(&outcome, &side, facts);
    if !stop {
        return Ok(combined);
    }
    let at = CapWhere {
        session_id: use_.session.session_id.clone(),
        facts: facts.clone(),
    };
    Ok(request
        .stop_cap
        .apply(combined, at, use_.ports.environment)
        .await)
}

crate::registry::exports! {
    "host/session-core-dispatch" {
        "dispatchBothForms" => async fallible |request: CoreDispatch, claude: Callback|
            dispatch_both_forms(request, claude).await;
    }
}
