//! The core's counterpart of `src/hooks/handlers/context.ts` at sha256:fb48d5f424fe, which stays
//! TypeScript: types only, so there is nothing to call across the boundary (docs/RUST.md, "What
//! stays TypeScript"). Only the runs the ported modules make are here: a command's run, and one
//! moved to the background (`hooks/handlers/command/async.rs`).

use serde::{Deserialize, Serialize};

use crate::registry::CorePromise;

/// A run that ended: the exit code and what the process wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitedRun {
    /// The exit code.
    pub exit_code: i32,
    /// What it wrote to standard output.
    pub stdout: String,
    /// What it wrote to standard error.
    pub stderr: String,
    /// The hook's own process ended while a process it started still held its stdout or stderr
    /// open (hooks.md, "Hooks that start a background process"): the capture is suspect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_held_open: Option<bool>,
}

/// A run that ended another way. `timed-out` discards the output ("Claude Code cancels a hook
/// that reaches its timeout, discarding the hook's output"); `not-started` is a program that did
/// not start (a missing script); `cancelled` is a process killed by the session (an abort).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ForegroundRun {
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
}

/// What a background hook reports once its process ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundResult {
    /// How the run ended.
    pub run: ForegroundRun,
    /// `asyncRewake` only: the text that wakes Claude, the stderr (stdout when stderr is empty)
    /// of a run that exited with code 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wake: Option<String>,
}

/// A hook running in the background: the engine goes on, and reads `done` when it settles.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename = "background")]
pub struct BackgroundRun {
    /// What it reports once its process ends.
    pub done: CorePromise<BackgroundResult>,
}
