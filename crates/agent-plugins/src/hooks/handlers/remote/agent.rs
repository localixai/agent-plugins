//! Port of `src/hooks/handlers/remote/agent.ts`, switched.
//!
//! An `agent` hook (hooks.md, "Agent-based hooks"): "like prompt-based hooks but with multi-turn
//! tool access": a subagent that "can use tools like Read, Grep, and Glob to investigate" and returns
//! `{ "ok": true/false }`. It has no `continueOnBlock` and no `impossible` ("don't support the
//! prompt-hook `impossible` field"). Skipping it on `PermissionRequest` is the select stage's.

use super::failure_run::{EntryEvent, without_port};
use super::model_run::{ModelCall, ModelHandled, run_model_hook};
use crate::hooks::handlers::context::RunContext;
use crate::hooks::handlers::timeouts::TimedEntry;
use crate::ports::model_runner::{AgentRequest, ModelRunner as _};

/// "Tools like Read, Grep, and Glob": the read-only investigation tools the docs name (judgment).
pub const AGENT_HOOK_TOOLS: [&str; 3] = ["Read", "Grep", "Glob"];

/// Runs one `agent` hook. `entry` is the registered handler, `context` the run's `RunContext`.
/// The port asks for it read-only (`readOnly: true`, `registry/remote/model_runner.rs`).
pub async fn run_agent(entry: &TimedEntry, context: RunContext) -> ModelHandled {
    let context = context.read().await;
    let Some(models) = context.ports.models else {
        let event = EntryEvent {
            event: entry.event.clone(),
        };
        return without_port(&event, "agent", "ModelRunner").into();
    };
    run_model_hook(
        entry,
        &context,
        ModelCall {
            impossible: false,
            call: |request| {
                models.run_agent(AgentRequest {
                    request,
                    tools: AGENT_HOOK_TOOLS.map(str::to_owned).to_vec(),
                    max_turns: 50,
                })
            },
        },
    )
    .await
}

crate::registry::exports! {
    "hooks/handlers/remote/agent" {
        "AGENT_HOOK_TOOLS" => ok || AGENT_HOOK_TOOLS;
        "runAgent" => async ok |entry: TimedEntry, context: RunContext| run_agent(&entry, context).await;
    }
}
