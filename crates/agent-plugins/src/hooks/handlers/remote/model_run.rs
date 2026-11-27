//! Ported from `src/hooks/handlers/remote/model-run.ts`; only the core uses it now, so it has no wrapper.
//!
//! What `prompt` and `agent` hooks share: the prompt with the hook input, the request the model
//! port gets, and the verdict read from its answer. They differ in the port call, the tools and
//! whether `impossible` counts, which the caller gives.

use serde::Serialize;

use super::failure_run::{EndedRun, HandlerResult, SkippedRun, ended_run};
use super::http::whole_ms;
use super::prompt_response::{
    ModelRun, ParsedVerdict, VerdictOptions, parse_verdict, response_instruction,
};
use super::prompt_text::compose_prompt;
use crate::format::issues::PluginIssue;
use crate::hooks::handlers::context::RunContextRead;
use crate::hooks::handlers::timeouts::{TimedEntry, handler_timeout_ms};
use crate::js::JsValue;
use crate::ports::failure::PortFailure;
use crate::ports::model_runner::{ModelAnswer, ModelPreference, ModelRequest};

/// A model call that ended without a verdict: `{ kind: 'model-failed', message }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelFailed {
    /// `model-failed`.
    pub kind: &'static str,
    /// Why.
    pub message: String,
}

fn model_failed(message: String) -> ModelHookRun {
    ModelHookRun::Failed(ModelFailed {
        kind: "model-failed",
        message,
    })
}

/// What a run of a `prompt` or `agent` hook was.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ModelHookRun {
    /// It did not run.
    Skipped(SkippedRun),
    /// `timed-out` or `cancelled`.
    Ended(EndedRun),
    /// The model gave no verdict.
    Failed(ModelFailed),
    /// The model's verdict.
    Model(ModelRun),
}

/// The raw run and the issues the run itself found (`HandlerResult`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelHandled {
    /// The run.
    pub run: ModelHookRun,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// A hook that did not run (`withoutPort`).
impl From<HandlerResult> for ModelHandled {
    fn from(without: HandlerResult) -> Self {
        Self {
            run: ModelHookRun::Skipped(without.run),
            issues: without.issues,
        }
    }
}

/// How a hook type calls the model.
pub struct ModelCall<F> {
    /// Reads `impossible` ("Agent hooks and other events ignore it").
    pub impossible: bool,
    /// The port call.
    pub call: F,
}

fn text_of(entry: &TimedEntry, key: &str) -> Option<String> {
    entry
        .handler
        .get(key)
        .and_then(JsValue::as_str)
        .map(str::to_owned)
}

/// Runs a `prompt` or `agent` hook through `call`.
pub async fn run_model_hook<F, R>(
    entry: &TimedEntry,
    context: &RunContextRead,
    ModelCall { impossible, call }: ModelCall<F>,
) -> ModelHandled
where
    F: FnOnce(ModelRequest) -> R,
    R: Future<Output = Result<ModelAnswer, PortFailure>>,
{
    let prompt = text_of(entry, "prompt").unwrap_or_default();
    let model = text_of(entry, "model");
    let timeout_ms = handler_timeout_ms(entry, &context.timeouts);
    let options = VerdictOptions { impossible };
    let answer = call(ModelRequest {
        prompt: format!(
            "{}\n\n{}",
            compose_prompt(&prompt, &context.input),
            response_instruction(options)
        ),
        model_preference: model.is_none().then_some(ModelPreference::SmallFast),
        model,
        timeout_ms: timeout_ms.map(whole_ms),
        signal: context.signal.clone(),
    })
    .await;
    let run = match answer {
        Err(failure) => ended_run(&failure).map_or_else(
            || model_failed(failure.message.clone()),
            ModelHookRun::Ended,
        ),
        Ok(answer) => match parse_verdict(&answer.text, options) {
            ParsedVerdict::Verdict { verdict } => ModelHookRun::Model(verdict),
            ParsedVerdict::Failure { failure } => model_failed(failure),
        },
    };
    ModelHandled {
        run,
        issues: Vec::new(),
    }
}
