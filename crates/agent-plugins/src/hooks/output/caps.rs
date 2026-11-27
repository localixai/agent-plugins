//! Port of `src/hooks/output/caps.ts`, switched.
//!
//! What each event honours of a hook's JSON output and what it discards (hooks.md, "JSON output":
//! "Every event accepts them, but some events discard them ... Each event's section says so"). One
//! row per event of the official union.

use indexmap::IndexMap;
use serde::Serialize;

use crate::registry::Set;

/// The groups of fields an event can discard.
pub const CAPPED_FIELDS: [&str; 5] = [
    // `continue` and `stopReason`.
    "continue",
    "systemMessage",
    "terminalSequence",
    // The top-level `decision` and `reason`.
    "decision",
    // `hookSpecificOutput`: `additionalContext` and the event's own fields.
    "specific",
];

/// What an event accepts and what it discards.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutputCap {
    /// The groups it honours.
    pub accepts: Set<String>,
    /// The groups it discards.
    pub discards: Set<String>,
}

fn cap_of(discarded: &[&str]) -> OutputCap {
    let text = |fields: Vec<&str>| Set {
        members: fields.into_iter().map(str::to_owned).collect(),
    };
    OutputCap {
        accepts: text(
            CAPPED_FIELDS
                .into_iter()
                .filter(|field| !discarded.contains(field))
                .collect(),
        ),
        discards: text(discarded.to_vec()),
    }
}

/// Events with no decision control: "No blocking or decision control".
const NO_DECISION: [&str; 1] = ["decision"];

/// The caps by event. `terminalSequence` is dropped nowhere here: "Claude Code writes the sequence
/// itself when it processes your hook's output, so the field works on events that discard
/// `systemMessage` and `continue`", and its two exceptions (non-interactive sessions, a command
/// `WorktreeCreate` hook) are not a matter of the event's output but of the host and handler.
/// Judgment (spec/judgments.md): that includes the events that "discard their JSON output fields".
const EVENT_CAPS: &[(&str, &[&str])] = &[
    ("PreToolUse", &[]),
    ("PostToolUse", &[]),
    ("PostToolUseFailure", &[]),
    ("PostToolBatch", &[]),
    ("UserPromptSubmit", &[]),
    ("UserPromptExpansion", &[]),
    ("Stop", &[]),
    ("SubagentStop", &[]),
    ("PermissionRequest", &NO_DECISION),
    ("PreModelSwitch", &[]),
    // "Claude Code shows the user any `systemMessage` your hook returns regardless of the decision".
    ("PostModelSwitch", &NO_DECISION),
    ("SubagentStart", &NO_DECISION),
    ("SessionStart", &NO_DECISION),
    // "On every exit code, Claude Code discards a Setup hook's JSON output fields".
    (
        "Setup",
        &["continue", "systemMessage", "decision", "specific"],
    ),
    // "Claude Code discards their `systemMessage` and `continue` fields".
    ("Notification", &["continue", "systemMessage", "decision"]),
    (
        "InstructionsLoaded",
        &["continue", "systemMessage", "decision", "specific"],
    ),
    (
        "SessionEnd",
        &["continue", "systemMessage", "decision", "specific"],
    ),
    // "Claude Code discards a PreCompact hook's `systemMessage` and `continue` fields."
    ("PreCompact", &["continue", "systemMessage"]),
    ("PostCompact", &["continue", "systemMessage", "decision"]),
    // "Run for notification and logging purposes only": only `terminalSequence` counts.
    (
        "StopFailure",
        &["continue", "systemMessage", "decision", "specific"],
    ),
    // "continue: false stops the teammate entirely"; no top-level decision.
    ("TeammateIdle", &NO_DECISION),
    ("TaskCompleted", &NO_DECISION),
    // "Claude Code ignores `continue: false` from this event".
    ("TaskCreated", &["continue"]),
    // "Claude Code acts on the blocking decision ... and discards `systemMessage` and `continue`."
    ("ConfigChange", &["continue", "systemMessage"]),
    // "Claude Code reads `watchPaths` and `systemMessage` ... and discards `continue`."
    ("CwdChanged", &["continue", "decision"]),
    ("FileChanged", &["continue", "decision"]),
    ("DirectoryAdded", &["continue", "decision"]),
    // "Claude Code acts on the hook's success and the returned path, and discards `systemMessage` and `continue`."
    ("WorktreeCreate", &["continue", "systemMessage", "decision"]),
    (
        "WorktreeRemove",
        &["continue", "systemMessage", "decision", "specific"],
    ),
    // "Claude Code discards `reason`, `systemMessage`, and `continue`"; `decision: "block"` declines.
    ("Elicitation", &["continue", "systemMessage"]),
    ("ElicitationResult", &["continue", "systemMessage"]),
    // "The only output this event reads is `hookSpecificOutput.retry`."
    (
        "PermissionDenied",
        &["continue", "systemMessage", "decision"],
    ),
    // "discards `systemMessage` and `continue`".
    ("MessageDisplay", &["continue", "systemMessage", "decision"]),
];

/// The cap of each event, in the package's order.
#[must_use]
pub fn output_caps() -> IndexMap<&'static str, OutputCap> {
    EVENT_CAPS
        .iter()
        .map(|(event, discarded)| (*event, cap_of(discarded)))
        .collect()
}

crate::registry::exports! {
    "hooks/output/caps" {
        "CAPPED_FIELDS" => ok || CAPPED_FIELDS;
        "OUTPUT_CAPS" => ok output_caps;
    }
}
