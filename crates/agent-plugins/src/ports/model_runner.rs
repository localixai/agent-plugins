//! The core's counterpart of `src/ports/model-runner.ts` at sha256:5450429c8a15, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! A model the crate asks: the hooks' `prompt` and `agent` handlers.

use serde::{Deserialize, Serialize};

use super::{BoxFuture, PortFailure, Signal};

/// What the model answered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelAnswer {
    /// Its text.
    pub text: String,
}

/// Which model a request leaves the host to pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelPreference {
    /// The host's small, fast model.
    #[serde(rename = "small-fast")]
    SmallFast,
}

/// One question to a model.
#[derive(Clone, Debug)]
pub struct ModelRequest {
    /// The prompt.
    pub prompt: String,
    /// The model named for it.
    pub model: Option<String>,
    /// The kind of model to use when none is named.
    pub model_preference: Option<ModelPreference>,
    /// After how long the host gives up.
    pub timeout_ms: Option<u64>,
    /// Its cancellation.
    pub signal: Signal,
}

/// A read-only agent run: a model with tools, for an `agent` hook handler.
#[derive(Clone, Debug)]
pub struct AgentRequest {
    /// The question.
    pub request: ModelRequest,
    /// The tools it may use.
    pub tools: Vec<String>,
    /// At most this many turns.
    pub max_turns: u32,
}

/// The host's models.
pub trait ModelRunner: Send + Sync {
    /// Asks a model once.
    fn evaluate(&self, request: ModelRequest) -> BoxFuture<'_, Result<ModelAnswer, PortFailure>>;
    /// Runs a read-only agent.
    fn run_agent(&self, request: AgentRequest) -> BoxFuture<'_, Result<ModelAnswer, PortFailure>>;
}
