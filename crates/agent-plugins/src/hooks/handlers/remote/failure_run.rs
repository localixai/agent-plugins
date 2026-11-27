//! Ported from `src/hooks/handlers/remote/failure-run.ts`; only the core uses it now, so it has no wrapper.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::ports::failure::PortFailure;

/// A run that did not give an answer: the shared `timed-out` and `cancelled` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct EndedRun {
    /// `timed-out` or `cancelled`.
    pub kind: &'static str,
}

/// The shared run a failure is, when it is one: a `timeout` is `timed-out` and an `aborted`
/// request is `cancelled`; any other failure is the type's own, which its handler names.
#[must_use]
pub fn ended_run(failure: &PortFailure) -> Option<EndedRun> {
    match failure.failure.as_str() {
        "timeout" => Some(EndedRun { kind: "timed-out" }),
        "aborted" => Some(EndedRun { kind: "cancelled" }),
        _ => None,
    }
}

/// The part of a registered handler a path reads.
#[derive(Clone, Debug, Deserialize)]
pub struct EntryEvent {
    /// The event it fires on.
    pub event: String,
}

/// The path of a handler's field in an issue: `hooks.PostToolUse.url`.
#[must_use]
pub fn remote_path(entry: &EntryEvent, field: &str) -> String {
    format!("hooks.{}.{field}", entry.event)
}

/// A run that never started, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkippedRun {
    /// `skipped`.
    pub kind: &'static str,
    /// The message of the issue.
    pub reason: String,
}

/// The raw run and the issues the run itself found (warnings, a refusal).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HandlerResult {
    /// The run.
    pub run: SkippedRun,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// A hook whose port the host did not give does not run: "a missing port is a `PluginIssue` for the
/// component that needed it" (docs/ARCHITECTURE.md, "Ports").
#[must_use]
pub fn without_port(entry: &EntryEvent, handler_type: &str, port: &str) -> HandlerResult {
    let message = format!(
        "A {handler_type} hook needs the host's {port}, which it did not give: the hook does not run."
    );
    let issues = vec![error(&remote_path(entry, "type"), &message)];
    HandlerResult {
        run: SkippedRun {
            kind: "skipped",
            reason: message,
        },
        issues,
    }
}
