//! Ported from `src/hooks/run-all.ts`; only the core uses it now, so it has no wrapper.
//!
//! The run stage of one dispatch: every selected handler runs through the run port and is read. A
//! run never rejects the dispatch: a port that throws is a hook that could not start, read through
//! the failure table like any other, so a sibling hook's block still holds. A `once` handler's hold
//! ends with its run (its process, for a background one), and a background delivery gets the cap
//! and escape of the foreground text.

mod one;

use std::sync::Arc;

use serde::Deserialize;

use super::contracts::MonitorTreatment;
use super::dispatch::config::{DispatchConfig, DispatchRequest};
use super::handlers::remote::concurrency::{limit_of, run_limited};
use super::handlers::timeouts::{TimedEntry, plan_timeouts};
use super::input::build::serialize_hook_input;
use super::interrupt::InterruptPlan;
use super::output::limit::{LimitRequest, limit_delivery};
use super::select::matcher_targets::{MatcherValue, value_of};
use super::select::once::Claims;
use super::verdict::debug_lines::Subject;
use super::verdict::interpret::{Delivery, interpret_subject};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::registry::{CorePromise, catch_host};
use one::{Ran, run_one};

/// A selected handler, under its serial, with how a Monitor call reached it.
#[derive(Clone, Debug)]
pub struct Selected {
    /// The registration's serial.
    pub serial: u32,
    /// The registered handler.
    pub entry: JsValue,
    /// Only for a Monitor call that reached it.
    pub monitor: Option<MonitorTreatment>,
}

/// What the run stage is given (`Planned`).
pub struct Planned {
    /// The handlers to run.
    pub selected: Vec<Selected>,
    /// The interrupt's plan.
    pub plan: InterruptPlan,
    /// The hook input.
    pub input: JsValue,
}

/// The input text of each handler, serialized once: a Monitor call that reached a `Bash` matcher is
/// a Bash call.
fn input_texts(input: &JsValue, selected: &[Selected]) -> (String, String) {
    let text = serialize_hook_input(input).unwrap_or_default();
    let as_bash = if selected.iter().all(|one| one.monitor.is_none()) {
        text.clone()
    } else {
        let mut call = input.as_object().cloned().unwrap_or_default();
        call.insert("tool_name".to_owned(), JsValue::from("Bash"));
        JsValue::Object(call).to_json_text()
    };
    (text, as_bash)
}

fn subject_of(
    request: &DispatchRequest,
    config: &DispatchConfig,
    one: &Selected,
    timeout_ms: Option<f64>,
) -> Subject {
    Subject {
        entry: one.entry.clone(),
        event: request.event.clone(),
        tool_name: crate::hooks::input::fact_read::text_fact_of(&request.facts, "tool_name"),
        facts: request.facts.clone(),
        parse: config.output.parser(),
        effects_ignored: one.monitor.is_some_and(|monitor| !monitor.effects_apply),
        timeout_ms,
        git_bash_missing: config.windows && config.default_shell == "powershell",
        teammate_stopped: request.task_completed_by.as_deref() == Some("teammate-turn"),
    }
}

/// Ends the `once` hold of each run and caps what a background run delivers. A foreground run
/// settles now, a background one when its process ends: hooks.md, "Common fields", consumes a
/// skill's `once` hook "after its first successful run", async completion included.
fn settled(
    request: &DispatchRequest,
    config: &DispatchConfig,
    claims: &Arc<dyn Claims>,
    one: &Selected,
    (mut verdict, delivery): (JsObject, Option<Delivery>),
) -> JsObject {
    let Some(delivery) = delivery else {
        let succeeded = verdict.get("succeeded").and_then(JsValue::as_bool) == Some(true);
        claims.settle(one.serial, &one.entry, succeeded);
        return verdict;
    };
    let limit = LimitRequest {
        signal: request.signal.clone(),
        scratch: config.scratch,
    };
    let (claims, serial, entry, event) = (
        Arc::clone(claims),
        one.serial,
        one.entry.clone(),
        request.event.clone(),
    );
    let done = CorePromise::new(async move {
        match catch_host(delivery).await {
            Ok(done) => {
                claims.settle(serial, &entry, done.exit_code == Some(0));
                limit_delivery(&event, done, &limit).await
            }
            Err(thrown) => {
                claims.settle(serial, &entry, false);
                thrown.rethrow().await
            }
        }
    });
    verdict.insert(
        "delivery".to_owned(),
        to_js_value(&done).unwrap_or_default(),
    );
    verdict
}

/// The run stage: every selected handler runs, then each run is read, in the selection's order.
pub async fn run_all(
    request: &DispatchRequest,
    config: &DispatchConfig,
    planned: Planned,
    claims: &Arc<dyn Claims>,
) -> Vec<JsObject> {
    let event = request.event.as_str();
    let timed: Vec<TimedEntry> = planned
        .selected
        .iter()
        .filter_map(|one| TimedEntry::deserialize(one.entry.clone()).ok())
        .collect();
    let timeouts = plan_timeouts(event, &timed, &config.environment, planned.plan.cap_ms).await;
    let (text, as_bash) = input_texts(&planned.input, &planned.selected);
    let matched_value = matched_value_of(request, config).await;
    let entries: Vec<JsValue> = planned
        .selected
        .iter()
        .map(|one| one.entry.clone())
        .collect();
    let planned_ref = &planned;
    let ran = run_limited(
        &entries,
        |index, _| {
            let one = planned_ref.selected.get(index);
            let text = if one.and_then(|one| one.monitor).is_some_and(|m| m.as_bash) {
                as_bash.clone()
            } else {
                text.clone()
            };
            let (timing, matched) = (&timeouts, matched_value.clone());
            Box::pin(async move {
                match one {
                    Some(one) => Some(
                        run_one(config, request, planned_ref, (timing, matched), (one, text)).await,
                    ),
                    None => None,
                }
            })
        },
        limit_of(None),
    )
    .await;
    let mut verdicts = Vec::new();
    for (one, ran) in planned.selected.iter().zip(ran) {
        let Some(Ran {
            run,
            issues: own,
            timeout_ms,
            facts,
        }) = ran
        else {
            continue;
        };
        let subject = subject_of(request, config, one, timeout_ms);
        let (mut verdict, delivery) = interpret_subject(&subject, run).await;
        verdict.insert("ran".to_owned(), facts);
        let mut issues = verdict
            .get("issues")
            .and_then(JsValue::as_array)
            .cloned()
            .unwrap_or_default();
        issues.extend(own);
        verdict.insert("issues".to_owned(), JsValue::Array(issues));
        verdicts.push(settled(request, config, claims, one, (verdict, delivery)));
    }
    verdicts
}

/// The value the event's matcher is tested against on this occurrence, when it has one.
async fn matched_value_of(request: &DispatchRequest, config: &DispatchConfig) -> Option<String> {
    match value_of(
        &request.event,
        &request.facts,
        config.model_catalog.as_ref(),
    )
    .await
    {
        MatcherValue::Filters(value) => value,
        MatcherValue::NoFilter => None,
    }
}
