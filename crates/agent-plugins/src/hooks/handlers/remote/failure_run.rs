//! Port of `src/hooks/handlers/remote/failure-run.ts`, switched.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::js::JsValue;
use crate::ports::failure::PortFailure;
use crate::registry::Callback;

/// A run that did not give an answer: the shared `timed-out` and `cancelled` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct EndedRun {
    /// `timed-out` or `cancelled`.
    pub kind: &'static str,
}

/// What a port's failure is to a run: the shared run of a timeout or an abort, or the type's own.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FailureRun {
    /// `timed-out` or `cancelled`.
    Ended(EndedRun),
    /// What the caller's `other` made of the message.
    Other(JsValue),
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

/// What a port's `PortFailure` is to a hook run. "Timeout: the hook is canceled, as described under
/// Timeouts", so a `timeout` is the shared `timed-out` run; an `aborted` request is a session abort,
/// the shared `cancelled` run. Any other failure is the type's own non-blocking failure, which the
/// caller names.
pub async fn failure_run(failure: &PortFailure, other: Callback) -> FailureRun {
    if let Some(ended) = ended_run(failure) {
        return FailureRun::Ended(ended);
    }
    FailureRun::Other(
        other
            .call::<JsValue>(vec![JsValue::from(failure.message.as_str())])
            .await,
    )
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

crate::registry::exports! {
    "hooks/handlers/remote/failure-run" {
        "failureRun" => async ok |failure: PortFailure, other: Callback| failure_run(&failure, other).await;
        "withoutPort" => ok |entry: EntryEvent, handler_type: String, port: String| without_port(&entry, &handler_type, &port);
    }
}
