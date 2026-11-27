//! The core's counterpart of `src/hooks/verdict/types.ts` at sha256:bc62e6ae89a2, which stays TypeScript:
//! types only, so there is nothing to call across the boundary (docs/RUST.md, "What stays
//! TypeScript"). Only the types the ported modules read are here: what a background hook hands
//! over (`hooks/output/background.rs`), the blocks, the merge of the outputs and a parse.

use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::js::JsObject;

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

/// How a block came about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockVia {
    /// Exit code 2.
    #[serde(rename = "exit2")]
    Exit2,
    /// A blocking decision.
    Decision,
    /// A failure that blocks.
    Failure,
    /// `continue: false`.
    ContinueFalse,
    /// A timeout that blocks.
    Timeout,
}

/// Who is shown the text of a block, as the docs route it: `claude` (the denial reason, the
/// feedback), `user` (the block message), both (`PostToolBatch`: "a warning in the transcript, and
/// it stays in the conversation"), or `none` (`ConfigChange`: "A blocked change surfaces no message
/// to you or to Claude"; `Elicitation`: "Claude Code doesn't show your stderr").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockRoute {
    /// Claude.
    Claude,
    /// The user.
    User,
    /// Both.
    UserAndClaude,
    /// No one.
    None,
}

/// The action is held back; `reason` is the text Claude or the user is given, as the docs route it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockInfo {
    /// How.
    pub via: BlockVia,
    /// Where `reason` and `message` are shown.
    pub to: BlockRoute,
    /// The hook's own words: its stderr, or the failure text. Empty when `to` is `none`.
    pub reason: String,
    /// The line the docs show. Empty when `to` is `none`.
    pub message: String,
    /// `to` is `none` and the docs send the hook's command and stderr to the debug log: that line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>,
    /// `WorktreeRemove` only: it fails "if the directory still exists afterward".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// A `<hook name> hook error` notice, verbatim as the docs show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookNotice {
    /// The handler.
    pub handler_id: String,
    /// The text.
    pub message: String,
}

/// Text from a hook that goes to Claude or to the user without blocking anything.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    /// The handler.
    pub handler_id: String,
    /// `claude` or `user`.
    pub to: String,
    /// The text.
    pub text: String,
}

/// How a merged block holds the action back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MergedVia {
    /// A blocking decision.
    Decision,
    /// `continue: false` where the event reads it as a block.
    ContinueFalse,
}

/// A decision of the merge that holds the action back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergedBlock {
    /// How it holds the action back.
    pub via: MergedVia,
    /// The handler whose output it is.
    pub by: String,
    /// The reason the output gave.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The merge of every hook's output on one event (hooks hold no knowledge of its rules).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergedOutput {
    /// The block of the merge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<MergedBlock>,
    /// Whether Claude goes on.
    #[serde(rename = "continue")]
    pub proceeds: bool,
    /// The reason a hook stopped Claude.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Shown to the user.
    pub system_messages: Vec<String>,
    /// Added to Claude's context.
    pub additional_context: Vec<String>,
    /// The terminal sequences to write.
    pub terminal_sequences: Vec<String>,
    /// The event's effects.
    pub effects: JsObject,
    /// What the merge dropped and why.
    pub issues: Vec<PluginIssue>,
}

/// The output stage's reading of one JSON value (`parseHookOutput`): the output, or why it failed.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    /// The validated output.
    #[serde(default)]
    pub output: Option<JsObject>,
    /// Why it failed.
    #[serde(default)]
    pub failure: Option<String>,
    /// Keys of a valid output the schema does not name, ignored.
    #[serde(default)]
    pub unrecognized_keys: Option<Vec<String>>,
}
