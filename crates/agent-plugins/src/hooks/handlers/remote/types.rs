//! The core's counterpart of `src/hooks/handlers/remote/types.ts` at sha256:f709316b7233, which stays
//! TypeScript: types only, so there is nothing to call across the boundary (docs/RUST.md, "What
//! stays TypeScript").
//!
//! What the remote handler types report (hooks.md, "HTTP hook fields", "MCP tool hook fields",
//! "Prompt-based hooks", "Agent-based hooks"). A run says what happened and never interprets it:
//! what a status, an `isError` or an `ok: false` does on an event is the interpreter's. A timeout,
//! a cancellation and a hook that did not run are the shared `timed-out`, `cancelled` and
//! `skipped` runs of `context.ts`, so every handler type is timed out and skipped the same way.

use serde::Deserialize;

/// The model's verdict, validated against the response schema (hooks.md, "Response schema").
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ModelRun {
    /// Whether the condition holds.
    pub ok: bool,
    /// Why it does not.
    #[serde(default)]
    pub reason: Option<String>,
    /// Prompt hooks only: the model judged the condition can never be satisfied.
    #[serde(default)]
    pub impossible: Option<bool>,
}
