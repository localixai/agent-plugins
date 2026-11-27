//! Ported from `src/hooks/run-facts.ts`; only the core uses it now, so it has no wrapper.
//!
//! What the run stage tells the host about each run it finishes (hooks.md, "Debug hooks": the line
//! names the event and the matched value, the command, the plugin, how the run ended and its
//! duration). The facts are the kit's; the line is the host's (Agent Plugins Extended §7.4.12 item 2,
//! Judgment J7.67).
//!
//! Judgment (spec/judgments.md): an HTTP request that got no answer `failed-to-start`, like a program
//! that did not start; an HTTP status, an MCP answer and a model's verdict, valid or not, `finished`.

use serde::Deserialize;

use super::contracts::HookSource;
use super::source_kind::{LabelledEntry, is_plugin_source, plugin_label};
use super::verdict::notices::handler_label;
use super::verdict::types::{RunEnding, RunFacts};
use crate::js::JsObject;
use crate::ports::clock::Clock as _;
use crate::registry::Port;

/// The kinds of a raw run (`RawRun['kind']`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum RunKind {
    /// `exited`
    #[serde(rename = "exited")]
    Exited,
    /// `timed-out`
    #[serde(rename = "timed-out")]
    TimedOut,
    /// `cancelled`
    #[serde(rename = "cancelled")]
    Cancelled,
    /// `not-started`
    #[serde(rename = "not-started")]
    NotStarted,
    /// `skipped`
    #[serde(rename = "skipped")]
    Skipped,
    /// `background`
    #[serde(rename = "background")]
    Background,
    /// `http`
    #[serde(rename = "http")]
    Http,
    /// `http-failed`
    #[serde(rename = "http-failed")]
    HttpFailed,
    /// `mcp`
    #[serde(rename = "mcp")]
    Mcp,
    /// `model`
    #[serde(rename = "model")]
    Model,
    /// `model-failed`
    #[serde(rename = "model-failed")]
    ModelFailed,
}

/// How each kind of run ended.
const fn ending_of(kind: RunKind) -> RunEnding {
    match kind {
        RunKind::Exited => RunEnding::Finished,
        RunKind::TimedOut => RunEnding::TimedOut,
        RunKind::Cancelled => RunEnding::Cancelled,
        RunKind::NotStarted => RunEnding::FailedToStart,
        RunKind::Skipped => RunEnding::Skipped,
        RunKind::Background => RunEnding::Background,
        RunKind::Http => RunEnding::Finished,
        RunKind::HttpFailed => RunEnding::FailedToStart,
        RunKind::Mcp => RunEnding::Finished,
        RunKind::Model => RunEnding::Finished,
        RunKind::ModelFailed => RunEnding::Finished,
    }
}

/// Only a run that reached its limit names the limit.
const fn gives_timeout(ending: RunEnding) -> bool {
    match ending {
        RunEnding::Finished => false,
        RunEnding::TimedOut => true,
        RunEnding::Cancelled => false,
        RunEnding::Background => false,
        RunEnding::FailedToStart => false,
        RunEnding::Skipped => false,
    }
}

/// When the run started, by the engine's clock; without one no duration is given.
pub async fn started_at(clock: Option<&Port>) -> Option<f64> {
    match clock {
        Some(clock) => Some(clock.now().await),
        None => None,
    }
}

/// The part of a registered handler the facts read.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactsEntry {
    /// The official handler object.
    pub handler: JsObject,
    /// Where it was declared.
    pub source: HookSource,
    /// The plugin's full id.
    #[serde(default)]
    pub plugin_id: Option<String>,
}

/// The part of a raw run the facts read.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactsRun {
    /// Which run.
    pub kind: RunKind,
    /// The exit code of a run that exited.
    #[serde(default)]
    pub exit_code: Option<f64>,
}

/// What the run's timing is read from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTiming {
    /// The engine's clock.
    #[serde(default)]
    pub clock: Option<Port>,
    /// When the run started.
    #[serde(default)]
    pub started: Option<f64>,
    /// The limit it was given.
    #[serde(default)]
    pub timeout_ms: Option<f64>,
    /// The value the matcher was tested against.
    #[serde(default)]
    pub matched_value: Option<String>,
}

/// How long the run took, when the engine has a clock.
async fn duration_of(timing: &RunTiming) -> Option<f64> {
    let ended = match (timing.started, timing.clock.as_ref()) {
        (Some(_), Some(clock)) => Some(clock.now().await),
        _ => None,
    };
    match (ended, timing.started) {
        (Some(ended), Some(started)) => Some(ended - started),
        _ => None,
    }
}

/// The facts of one finished run.
pub async fn run_facts_of(entry: &FactsEntry, run: &FactsRun, timing: &RunTiming) -> RunFacts {
    let ending = ending_of(run.kind);
    let plugin = if is_plugin_source(&entry.source) {
        plugin_label(&LabelledEntry {
            plugin_id: entry.plugin_id.clone(),
            source: entry.source.clone(),
        })
    } else {
        None
    };
    RunFacts {
        matched_value: timing.matched_value.clone(),
        handler: handler_label(&entry.handler),
        plugin,
        ending,
        exit_code: run.exit_code,
        timeout_ms: if gives_timeout(ending) {
            timing.timeout_ms
        } else {
            None
        },
        duration_ms: duration_of(timing).await,
    }
}
