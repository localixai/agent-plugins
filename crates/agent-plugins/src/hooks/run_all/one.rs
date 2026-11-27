//! One selected handler's run, before it is read: its context, the guard that makes a throwing run
//! port a hook that could not start, and the facts of how it ran.

use serde::Deserialize;

use super::{Planned, Selected};
use crate::hooks::dispatch::config::{DispatchConfig, DispatchRequest};
use crate::hooks::handlers::registry::{HandlerRun, RunOf, run_handler};
use crate::hooks::handlers::timeouts::{TimedEntry, TimeoutPlan, handler_timeout_ms};
use crate::hooks::run_facts::{FactsEntry, FactsRun, RunTiming, run_facts_of, started_at};
use crate::hooks::verdict::interpret::RawRun;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::registry::{catch_host, detach};

fn context_of(
    config: &DispatchConfig,
    request: &DispatchRequest,
    timeouts: &TimeoutPlan,
    input: String,
) -> JsValue {
    let mut context = JsObject::from_iter([
        ("ports".to_owned(), config.run_ports.clone()),
        ("input".to_owned(), JsValue::from(input)),
        ("cwd".to_owned(), request.session_text("cwd")),
        ("sessionId".to_owned(), request.session_text("sessionId")),
        ("windows".to_owned(), JsValue::Bool(config.windows)),
        (
            "defaultShell".to_owned(),
            JsValue::from(config.default_shell.as_str()),
        ),
        (
            "timeouts".to_owned(),
            to_js_value(timeouts).unwrap_or_default(),
        ),
        (
            "signal".to_owned(),
            to_js_value(&request.signal).unwrap_or_default(),
        ),
    ]);
    if let Some(level) = request
        .session
        .get("effort")
        .and_then(|effort| effort.get("level"))
    {
        context.insert("effort".to_owned(), level.clone());
    }
    context.extend(config.run_options.clone());
    JsValue::Object(context)
}

/// One handler's run. "you cancel the hooks at once": a hook not started when the signal fired (one
/// waiting for a slot among the prompt and agent hooks) does not start. A run port that throws is a
/// hook that could not run (judgment, spec/judgments.md).
async fn run_guarded(entry: &JsValue, context: JsValue, aborted: bool) -> HandlerRun {
    if aborted {
        return cancelled();
    }
    match catch_host(run_handler(entry, &context)).await {
        Ok(run) => run,
        Err(thrown) => run_of(&[
            ("kind", JsValue::from("not-started")),
            (
                "message",
                JsValue::from(format!("Hook could not run: {}", thrown.message)),
            ),
        ]),
    }
}

fn run_of(members: &[(&str, JsValue)]) -> HandlerRun {
    let run = members
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect();
    HandlerRun {
        run: RunOf::Data(JsValue::Object(run)),
        issues: Vec::new(),
    }
}

fn cancelled() -> HandlerRun {
    run_of(&[("kind", JsValue::from("cancelled"))])
}

/// What one run came to before it is read.
pub(super) struct Ran {
    pub run: RawRun,
    pub issues: Vec<JsValue>,
    pub timeout_ms: Option<f64>,
    pub facts: JsValue,
}

/// The run as the verdict reads it. A background run is handed to the host at once (`detach`), so
/// its process starts while the dispatch goes on, as the TypeScript's did when its run was called.
async fn raw_run_of(run: RunOf) -> RawRun {
    match run {
        RunOf::Background(done) => RawRun::Background {
            done: detach(done).await,
        },
        RunOf::Data(run) => RawRun::deserialize(run).unwrap_or(RawRun::NotStarted {
            message: "Hook could not run: its run could not be read".to_owned(),
        }),
    }
}

/// The run's kind and exit code, as its facts read them.
fn facts_run_of(run: &RunOf) -> Option<FactsRun> {
    match run {
        RunOf::Background(_) => FactsRun::deserialize(JsValue::Object(JsObject::from_iter([(
            "kind".to_owned(),
            JsValue::from("background"),
        )])))
        .ok(),
        RunOf::Data(run) => FactsRun::deserialize(run.clone()).ok(),
    }
}

pub(super) async fn run_one(
    config: &DispatchConfig,
    request: &DispatchRequest,
    planned: &Planned,
    timing: (&TimeoutPlan, Option<String>),
    (one, text): (&Selected, String),
) -> Ran {
    let (timeouts, matched_value) = timing;
    let clock = config.clock;
    let started = started_at(clock.as_ref()).await;
    let result = if planned.plan.cancel {
        cancelled()
    } else {
        let context = context_of(config, request, timeouts, text);
        run_guarded(&one.entry, context, request.signal.is_aborted()).await
    };
    let timeout_ms = TimedEntry::deserialize(one.entry.clone())
        .ok()
        .and_then(|timed| handler_timeout_ms(&timed, timeouts));
    let entry = FactsEntry::deserialize(one.entry.clone()).ok();
    let facts = match (entry, facts_run_of(&result.run)) {
        (Some(entry), Some(run)) => {
            let timing = RunTiming {
                clock,
                started,
                timeout_ms,
                matched_value,
            };
            to_js_value(&run_facts_of(&entry, &run, &timing).await).unwrap_or_default()
        }
        _ => JsValue::default(),
    };
    Ran {
        run: raw_run_of(result.run).await,
        issues: result.issues,
        timeout_ms,
        facts,
    }
}
