//! Ported from `src/hooks/handlers/remote/prompt.ts`; only the core uses it now, so it has no wrapper.
//!
//! A `prompt` hook (hooks.md, "Prompt-based hooks"): the hook input and the prompt go to a model
//! and "the LLM responds with structured JSON containing a decision". What `ok: false` does on an
//! event is `ok-false.ts`.

use super::failure_run::{EntryEvent, without_port};
use super::model_run::{ModelCall, ModelHandled, run_model_hook};
use crate::hooks::handlers::context::RunContext;
use crate::hooks::handlers::timeouts::TimedEntry;
use crate::ports::model_runner::ModelRunner as _;

/// Runs one `prompt` hook. `entry` is the registered handler, `context` the run's `RunContext`.
pub async fn run_prompt(entry: &TimedEntry, context: RunContext) -> ModelHandled {
    let context = context.read().await;
    let Some(models) = context.ports.models else {
        let event = EntryEvent {
            event: entry.event.clone(),
        };
        return without_port(&event, "prompt", "ModelRunner").into();
    };
    run_model_hook(
        entry,
        &context,
        ModelCall {
            impossible: true,
            call: |request| models.evaluate(request),
        },
    )
    .await
}
