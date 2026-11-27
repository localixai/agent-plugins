//! Port of `src/hooks/verdict/interpret.ts`, switched.
//!
//! Reads one handler's raw run as the docs read it (hooks.md, "Exit code output", "HTTP response
//! handling", "Timeouts"): one reader per kind of run, a registry over the union so a run the run
//! stage learns to report is a compile error here.
//!
//! The context holds the output stage's parser, a function, so it crosses as the host's object and
//! its members are read once, into the run's `Subject`. A background run's `done` is the host's
//! promise; the verdict's `delivery` is a promise of the core's that settles with what it hands
//! over.

use serde::Deserialize;

use super::background::delivery_of;
use super::debug_lines::{Parser, Subject};
use super::exited::interpret_exited;
use super::http_response::{interpret_http, interpret_http_failed};
use super::model_verdict::{interpret_model, interpret_model_failed};
use super::notices::{TIMED_OUT, timed_out_discarded};
use super::on_failure::{Failure, FailureCause, failure_verdict, value_of};
use super::types::{BackgroundDelivery, HookNotice};
use crate::hooks::handlers::context::{BackgroundResult, ExitedRun};
use crate::hooks::handlers::remote::types::ModelRun;
use crate::hooks::input::fact_read::text_fact_of;
use crate::js::{JsObject, JsValue};
use crate::ports::BoxFuture;
use crate::registry::{Callback, CorePromise, Detached, Port};

/// What running any hook returns (`RawRun`): the command runs and the runs of the remote types.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum RawRun {
    /// It exited.
    #[serde(rename = "exited")]
    Exited(ExitedRun),
    /// It reached its timeout.
    #[serde(rename = "timed-out")]
    TimedOut,
    /// It did not start.
    #[serde(rename = "not-started")]
    NotStarted {
        /// Why.
        message: String,
    },
    /// The session killed it.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// It did not run.
    #[serde(rename = "skipped")]
    Skipped {
        /// The line the conversation view shows for it, when the docs give one.
        #[serde(default)]
        notice: Option<String>,
    },
    /// It runs in the background.
    #[serde(rename = "background")]
    Background {
        /// What it reports once its process ends.
        done: Detached<BackgroundResult>,
    },
    /// An HTTP hook's response.
    #[serde(rename = "http")]
    Http {
        /// The status.
        status: f64,
        /// The body.
        body: String,
    },
    /// An HTTP hook whose request failed.
    #[serde(rename = "http-failed")]
    HttpFailed {
        /// Why.
        message: String,
    },
    /// An `mcp_tool` hook's answer.
    #[serde(rename = "mcp")]
    Mcp {
        /// The tool answered `isError: true`.
        #[serde(rename = "isError")]
        is_error: bool,
        /// Its text content.
        text: String,
    },
    /// A `prompt` or `agent` hook's verdict.
    #[serde(rename = "model")]
    Model(ModelRun),
    /// The model did not answer.
    #[serde(rename = "model-failed")]
    ModelFailed {
        /// Why.
        message: String,
    },
}

fn inactive(subject: &Subject, status: &str) -> JsObject {
    JsObject::from_iter([
        ("handler".to_owned(), subject.entry.clone()),
        ("status".to_owned(), JsValue::from(status)),
        ("succeeded".to_owned(), JsValue::Bool(false)),
        ("issues".to_owned(), JsValue::Array(vec![])),
    ])
}

/// The events whose docs give a timed-out hook a notice that says its output was discarded.
const NOTICES_DISCARD: [&str; 1] = ["UserPromptSubmit"];

/// "A `UserPromptSubmit` command, HTTP, or MCP tool hook that reaches its timeout is canceled and
/// its output ... is discarded."
const DISCARDS_ON_TIMEOUT: [(&str, bool); 5] = [
    ("command", true),
    ("http", true),
    ("mcp_tool", true),
    ("prompt", false),
    ("agent", false),
];

fn timeout_text(subject: &Subject) -> String {
    let handler = subject.handler();
    let kind = handler.get("type").and_then(JsValue::as_str);
    let discards = DISCARDS_ON_TIMEOUT
        .iter()
        .any(|(name, discards)| Some(*name) == kind && *discards);
    if NOTICES_DISCARD.contains(&subject.event.as_str()) && discards {
        timed_out_discarded(subject.timeout_ms)
    } else {
        TIMED_OUT.to_owned()
    }
}

fn failure(cause: FailureCause, text: &str, detail: &str) -> Failure {
    Failure {
        cause,
        text: text.to_owned(),
        detail: detail.to_owned(),
    }
}

async fn read_run(subject: &Subject, run: RawRun) -> JsObject {
    match run {
        RawRun::Exited(exited) => interpret_exited(subject, &exited).await,
        RawRun::TimedOut => failure_verdict(
            subject,
            &failure(FailureCause::Timeout, &timeout_text(subject), ""),
        ),
        RawRun::NotStarted { message } => failure_verdict(
            subject,
            &failure(FailureCause::CannotStart, &message, &message),
        ),
        RawRun::Cancelled => inactive(subject, "cancelled"),
        RawRun::Skipped { notice } => {
            let mut verdict = inactive(subject, "skipped");
            if let Some(message) = notice {
                let notice = HookNotice {
                    handler_id: subject.entry_id(),
                    message,
                };
                verdict.insert("notice".to_owned(), value_of(&notice));
            }
            verdict
        }
        RawRun::Background { .. } => inactive(subject, "background"),
        RawRun::Http { status, body } => interpret_http(subject, status, &body).await,
        RawRun::HttpFailed { message } => interpret_http_failed(subject, &message),
        // "Claude Code reads the tool's text content the same way it reads command-hook stdout ...
        // If the tool returns `isError: true`, the hook produces a non-blocking error and
        // execution continues."
        RawRun::Mcp { is_error, text } => {
            if is_error {
                failure_verdict(subject, &failure(FailureCause::ToolError, &text, &text))
            } else {
                let exited = ExitedRun {
                    exit_code: 0,
                    stdout: text,
                    stderr: String::new(),
                    output_held_open: None,
                };
                interpret_exited(subject, &exited).await
            }
        }
        RawRun::Model(model) => interpret_model(subject, &model),
        RawRun::ModelFailed { message } => interpret_model_failed(subject, &message),
    }
}

/// What the run of `entry` on `event` came to: the context's members as the host gave them.
async fn subject_of(entry: JsValue, context: Port) -> Subject {
    let facts = context.member::<JsValue>("facts").await;
    Subject {
        entry,
        event: context.member::<String>("event").await,
        tool_name: text_fact_of(&facts, "tool_name"),
        facts,
        parse: Parser::Host(context.member::<Callback>("parse").await),
        timeout_ms: context.member::<Option<f64>>("timeoutMs").await,
        effects_ignored: context.member::<Option<bool>>("effectsIgnored").await == Some(true),
        git_bash_missing: context.member::<Option<bool>>("gitBashMissing").await == Some(true),
        teammate_stopped: context.member::<Option<bool>>("teammateStopped").await == Some(true),
    }
}

/// What the run of `entry` came to.
pub async fn interpret_run(entry: JsValue, run: RawRun, context: Port) -> JsObject {
    let (mut verdict, delivery) = interpret_subject(&subject_of(entry, context).await, run).await;
    if let Some(delivery) = delivery {
        verdict.insert("delivery".to_owned(), value_of(&CorePromise::new(delivery)));
    }
    verdict
}

/// What a background run hands over once its process ends.
pub type Delivery = BoxFuture<'static, BackgroundDelivery>;

/// What a run came to, for the subject the core built, and a background run's delivery, which the
/// caller makes the verdict's `delivery` (the dispatch first wraps it, `hooks/run_all.rs`).
pub async fn interpret_subject(subject: &Subject, run: RawRun) -> (JsObject, Option<Delivery>) {
    match run {
        RawRun::Background { done } => {
            let handler_id = subject.entry_id();
            let delivery: Delivery =
                Box::pin(async move { delivery_of(&handler_id, &done.settled().await) });
            (inactive(subject, "background"), Some(delivery))
        }
        run @ (RawRun::Exited(_)
        | RawRun::TimedOut
        | RawRun::NotStarted { .. }
        | RawRun::Cancelled
        | RawRun::Skipped { .. }
        | RawRun::Http { .. }
        | RawRun::HttpFailed { .. }
        | RawRun::Mcp { .. }
        | RawRun::Model(_)
        | RawRun::ModelFailed { .. }) => (interpret_read(subject, run).await, None),
    }
}

async fn interpret_read(subject: &Subject, run: RawRun) -> JsObject {
    let exit_code = if let RawRun::Exited(exited) = &run {
        Some(exited.exit_code)
    } else {
        None
    };
    let mut verdict = read_run(subject, run).await;
    if let Some(exit_code) = exit_code {
        verdict.insert("exitCode".to_owned(), JsValue::Number(f64::from(exit_code)));
    }
    verdict
}

crate::registry::exports! {
    "hooks/verdict/interpret" {
        "interpretRun" => async ok |entry: JsValue, run: RawRun, context: Port|
            interpret_run(entry, run, context).await;
    }
}
