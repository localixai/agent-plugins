//! The core's counterpart of `src/hooks/verdict/types.ts` at sha256:bc62e6ae89a2, which stays TypeScript:
//! types only, so there is nothing to call across the boundary (docs/RUST.md, "What stays
//! TypeScript"). Only the types the ported modules read are here: what a background hook hands
//! over (`hooks/output/background.rs`).

use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;

/// How a background hook ended (hooks.md, "How async hooks execute"): `cancelled` a hook killed
/// at teardown ("in non-interactive mode with the `-p` flag, Claude Code kills any async hook
/// still running at teardown and finalizes it with outcome `cancelled`"); `timed-out` an
/// `asyncRewake` hook that reached its timeout; `failed` one that did not start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackgroundOutcome {
    /// It ran to its end.
    Completed,
    /// Killed at teardown.
    Cancelled,
    /// An `asyncRewake` hook that reached its timeout.
    TimedOut,
    /// It did not start.
    Failed,
}

/// What `async` and `asyncRewake` hooks hand over once their process ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundDelivery {
    /// The handler's id.
    pub handler_id: String,
    /// How it ended.
    pub outcome: BackgroundOutcome,
    /// A completed command's exit code: a host consumes a `once` hook only after success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// `additionalContext` of the JSON response, for the next turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
    /// `systemMessage` of the JSON response, for Claude on the next turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    /// `asyncRewake` after exit 2: wakes Claude with this text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wake: Option<String>,
    /// The debug warnings of the response: each field of the wrong type that was dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issues: Option<Vec<PluginIssue>>,
}

/// How a run ended, for the host's debug line (`RunFacts.ending`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunEnding {
    /// It ran to its end.
    Finished,
    /// It reached its limit.
    TimedOut,
    /// The session killed it.
    Cancelled,
    /// It went to the background.
    Background,
    /// It did not start.
    FailedToStart,
    /// It did not run.
    Skipped,
}

/// The facts of a finished run the host writes its debug line from (Agent Plugins Extended §7.4.12
/// item 2, Judgment J7.67): the line's text, timestamp and level are the host's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunFacts {
    /// The value the matcher was tested against (`Write`, `startup`); absent on an event without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_value: Option<String>,
    /// The handler as the docs name it: a command and its arguments, a URL, `server:tool`, or the type.
    pub handler: String,
    /// The plugin the hook came from, when it came from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// How the run ended.
    pub ending: RunEnding,
    /// The exit code of a command hook whose process ran to the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<f64>,
    /// `timed-out`: the limit it reached, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<f64>,
    /// From the start of the run to its end, by the engine's `clock`; absent without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
}
