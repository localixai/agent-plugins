//! Ported from `src/hooks/dispatch.ts`; only the core uses it now, so it has no wrapper.
//!
//! One dispatch: select the handlers, build the input, run them through the ports, read each run,
//! merge the outputs. The TypeScript passed the stages in so each could be tested alone; here the
//! stages are the core's own, but for the output stage a host may give (`HookEngineOptions.output`).

pub mod config;

use std::sync::Arc;

use serde::Deserialize;

use super::assemble::{assemble, idle_outcome, run_facts_of_verdict};
use super::input::build::build_hook_input;
use super::interrupt::{interrupt_plan, with_interrupt};
use super::output::watch_paths::with_resolved_watch_paths;
use super::run_all::{Planned, Selected, run_all};
use super::select::once::Claims;
use super::select::select::{SelectRequest, select_handlers};
use super::verdict::blocked_prompt::with_original_prompt;
use super::verdict::stop_block_cap::is_tool_call;
use crate::hooks::contracts::HookSession;
use crate::hooks::handlers::status::observe;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, promise_all};
use crate::ports::BoxFuture;
use crate::registry::Callback;
use config::{DispatchConfig, DispatchRequest};

/// What the merge reads of one verdict (`TaggedOutput`).
fn tagged(verdict: &JsObject) -> JsObject {
    let mut output = JsObject::new();
    output.insert(
        "handler".to_owned(),
        verdict.get("handler").cloned().unwrap_or_default(),
    );
    for name in ["output", "plainText", "stdout"] {
        if let Some(value) = verdict.get(name).filter(|value| !value.is_undefined()) {
            output.insert(name.to_owned(), value.clone());
        }
    }
    if verdict.get("effectsIgnored").and_then(JsValue::as_bool) == Some(true) {
        output.insert("effectsIgnored".to_owned(), JsValue::Bool(true));
    }
    if verdict
        .get("block")
        .is_some_and(|block| !block.is_undefined())
    {
        output.insert("blocked".to_owned(), JsValue::Bool(true));
    }
    output
}

/// What the observer is told of one finished run.
fn finished_info(event: &str, verdict: &JsObject) -> JsValue {
    let mut info = run_facts_of_verdict(verdict);
    info.insert("event".to_owned(), JsValue::from(event));
    let handler_id = verdict.get("handler").and_then(|handler| handler.get("id"));
    info.insert(
        "handlerId".to_owned(),
        handler_id.cloned().unwrap_or_default(),
    );
    info.insert(
        "status".to_owned(),
        verdict.get("status").cloned().unwrap_or_default(),
    );
    if let Some(code) = verdict.get("exitCode").filter(|code| !code.is_undefined()) {
        info.insert("exitCode".to_owned(), code.clone());
    }
    JsValue::Object(info)
}

/// Tells the observer how each run finished and the notice it raised, in order; what it throws is
/// dropped (`observe`).
async fn report(config: &DispatchConfig, event: &str, verdicts: &[JsObject]) {
    let Some(observer) = config.observer else {
        return;
    };
    let (finished, notice) = crate::js::promise_all_pair(
        Box::pin(observer.member::<Option<Callback>>("finished")),
        Box::pin(observer.member::<Option<Callback>>("notice")),
    )
    .await;
    let mut calls: Vec<BoxFuture<'_, ()>> = Vec::new();
    for verdict in verdicts {
        if finished.is_some() {
            let info = finished_info(event, verdict);
            calls.push(Box::pin(observe(
                observer.method::<JsValue>("finished", vec![info]),
            )));
        }
        if let (Some(_), Some(said)) = (notice, verdict.get("notice")) {
            let said = said.clone();
            calls.push(Box::pin(observe(
                observer.method::<JsValue>("notice", vec![said]),
            )));
        }
    }
    promise_all(calls).await;
}

/// "Claude Code writes the sequence only in an interactive session ... In non-interactive mode with
/// the `-p` flag and in the Agent SDK, it ignores the field" (hooks.md, "Emit terminal
/// notifications"). Whether the interface is on screen at that moment is the host's.
fn with_terminal(mut outcome: JsObject, session: &JsValue) -> JsObject {
    let interactive = session.get("interactive").and_then(JsValue::as_bool) == Some(true);
    let empty = outcome
        .get("terminalSequences")
        .and_then(JsValue::as_array)
        .is_none_or(Vec::is_empty);
    if !interactive && !empty {
        outcome.insert("terminalSequences".to_owned(), JsValue::Array(Vec::new()));
    }
    outcome
}

/// "When the `TaskUpdate` tool triggered the event, Claude Code ignores `continue: false`" (hooks.md,
/// "TaskCompleted decision control"); when a teammate's finished turn did, it stops the teammate.
/// With the trigger known the outcome says which; without it `effects.stopCondition` is left to the
/// host.
fn with_task_trigger(mut outcome: JsObject, request: &DispatchRequest) -> JsObject {
    let Some(by) = request.task_completed_by.as_deref() else {
        return outcome;
    };
    let Some(JsValue::Object(effects)) = outcome.get_mut("effects") else {
        return outcome;
    };
    if effects
        .get("stopCondition")
        .is_none_or(JsValue::is_undefined)
    {
        return outcome;
    }
    effects.shift_remove("stopCondition");
    if by == "teammate-turn" {
        return outcome;
    }
    outcome.shift_remove("stopReason");
    outcome.insert("continue".to_owned(), JsValue::Bool(true));
    outcome
}

/// What the interrupt, the prompt and the Stop cap leave of an outcome (hooks.md, "Hook lifecycle").
async fn finished(
    outcome: JsObject,
    request: &DispatchRequest,
    config: &DispatchConfig,
    capped: bool,
) -> JsObject {
    let outcome = with_task_trigger(with_terminal(outcome, &request.session), request);
    let outcome = with_resolved_watch_paths(outcome, &request.session_string("cwd"));
    let quoted = with_original_prompt(outcome, &request.facts);
    let interrupted = with_interrupt(JsValue::Object(quoted), request.signal.is_aborted());
    let interrupted = interrupted.as_object().cloned().unwrap_or_default();
    config.capped(interrupted, request, capped).await
}

/// The selected entries under their serials, with how a Monitor call reached each.
fn selected_of(
    entries: &[(u32, JsValue)],
    selection: &super::select::select::Selection,
) -> Vec<(u32, JsValue)> {
    selection
        .selected
        .iter()
        .filter_map(|index| entries.get(*index).cloned())
        .collect()
}

/// One dispatch over `entries`, each under its serial; `capped` is whether the Stop cap counts it
/// (a session counts a `Stop` its core handlers share itself, Agent Plugins Extended §18.1.2, J18.9).
///
/// # Errors
///
/// The facts are not an input the event's schema reads (`buildHookInput` throws).
pub async fn dispatch_event(
    entries: Vec<(u32, JsValue)>,
    request: DispatchRequest,
    config: &DispatchConfig,
    claims: &Arc<dyn Claims>,
    capped: bool,
) -> Result<JsObject, String> {
    let event = request.event.clone();
    let session = HookSession::deserialize(request.session.clone()).unwrap_or_default();
    // "The count of consecutive continuations resets each time Claude calls a tool": hook or no hook.
    if is_tool_call(&event) && capped {
        config.reset_cap(&request).await;
    }
    // "until the next CwdChanged event, when Claude Code clears them": whether or not a hook is there.
    config.before_event(&event, &request.signal).await;
    let values: Vec<JsValue> = entries.iter().map(|(_, entry)| entry.clone()).collect();
    let selection = select_handlers(
        &values,
        &SelectRequest {
            event: event.clone(),
            facts: request.facts.clone(),
            session: session.clone(),
            if_evaluator: config.if_evaluator,
            model_catalog: config.model_catalog,
        },
    )
    .await;
    let facts = request.facts.as_object().cloned().unwrap_or_default();
    let built = build_hook_input(&event, &facts, &session)?;
    let mut issues: Vec<JsValue> = to_js_value(&selection.issues)
        .ok()
        .and_then(|issues| issues.as_array().cloned())
        .unwrap_or_default();
    issues.extend(
        to_js_value(&built.issues)
            .ok()
            .and_then(|issues| issues.as_array().cloned())
            .unwrap_or_default(),
    );
    // A `once` handler another dispatch runs, or a run spent, is left out (`OnceClaims`).
    let taken = if built.input.is_undefined() {
        Vec::new()
    } else {
        claims.take(selected_of(&entries, &selection))
    };
    if taken.is_empty() {
        let idle = idle_outcome(&event, issues);
        return Ok(config.capped(idle, &request, capped).await);
    }
    let monitor_of = |serial: u32| {
        selection.monitor.iter().find_map(|(index, treatment)| {
            (entries.get(*index).map(|(at, _)| *at) == Some(serial)).then_some(*treatment)
        })
    };
    let selected = taken
        .into_iter()
        .map(|(serial, entry)| Selected {
            serial,
            entry,
            monitor: monitor_of(serial),
        })
        .collect();
    let plan = interrupt_plan(&event, request.interrupt.as_ref(), &config.environment).await;
    let planned = Planned {
        selected,
        plan,
        input: built.input,
    };
    let verdicts = run_all(&request, config, planned, claims).await;
    report(config, &event, &verdicts).await;
    let outputs = verdicts.iter().map(tagged).collect();
    let limited = config.output.limit(&event, outputs, &request.signal).await;
    let merged = config
        .output
        .merge(&event, limited.outputs, &request.facts)
        .await;
    issues.extend(limited.issues);
    let outcome = assemble(&event, &verdicts, &merged, issues);
    Ok(finished(outcome, &request, config, capped).await)
}
