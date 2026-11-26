//! Port of `src/hooks/core/output.ts`, switched.
//!
//! What a core handler's process came to (core §7.3.2 item 5, §7.3.3): a status other than `0`, a
//! timeout or a process that never started is a failure; on status `0`, no bytes or only JSON
//! whitespace is no opinion, and anything but exactly one JSON object of one of the event's closed
//! output shapes is a failure, `{}` included. Exit status 2 is a failure like any other (Agent
//! Plugins Extended §18.1.2, "Failures": never a block), and plain text is never context.

use serde::Serialize;

use super::types::{CoreDecision, CoreResult};
use crate::format::core::hooks::CoreHookEvent;
use crate::hooks::verdict::notices::{
    NO_STDERR, NON_BLOCKING_STATUS, TIMED_OUT, first_line, timed_out_discarded,
};
use crate::js::json_parse::json_parse;
use crate::js::{JsObject, JsValue, property_order};
use crate::ports::command_runner::CommandResult;

/// A field of a closed output shape: a nonempty string, or one fixed value.
#[derive(Clone, Copy)]
enum Field {
    Text,
    Is(&'static str),
}

type Shape = Vec<(&'static str, Field)>;

fn context() -> Shape {
    vec![("additional_context", Field::Text)]
}

fn decided(value: &'static str, reason: bool) -> Shape {
    if reason {
        vec![("decision", Field::Is(value)), ("reason", Field::Text)]
    } else {
        vec![("decision", Field::Is(value))]
    }
}

/// The output schemas of `schemas/1.1.0/hooks.schema.json`, every one closed.
fn shapes(event: CoreHookEvent) -> Vec<Shape> {
    match event {
        CoreHookEvent::SessionStart | CoreHookEvent::PostToolUse => vec![context()],
        CoreHookEvent::UserPromptSubmit => vec![context(), decided("deny", true)],
        CoreHookEvent::PreToolUse => vec![decided("allow", false), decided("deny", true)],
        CoreHookEvent::Stop => vec![decided("continue", true)],
    }
}

fn fits(field: Field, value: &JsValue) -> bool {
    value.as_str().is_some_and(|text| match field {
        Field::Text => !text.is_empty(),
        Field::Is(fixed) => text == fixed,
    })
}

fn matches(shape: &Shape, output: &JsObject) -> bool {
    shape.len() == property_order(output).len()
        && shape
            .iter()
            .all(|(key, field)| fits(*field, output.get(*key).unwrap_or(&JsValue::Undefined)))
}

fn decision_of(value: &JsValue) -> Option<CoreDecision> {
    match value.as_str()? {
        "allow" => Some(CoreDecision::Allow),
        "deny" => Some(CoreDecision::Deny),
        "continue" => Some(CoreDecision::Continue),
        _ => None,
    }
}

fn result_of(output: &JsObject) -> CoreResult {
    let member = |key: &str| output.get(key).unwrap_or(&JsValue::Undefined);
    if let Some(context) = member("additional_context").as_str() {
        return CoreResult::Context {
            context: context.to_owned(),
        };
    }
    CoreResult::Decision {
        decision: decision_of(member("decision")).unwrap_or(CoreDecision::Continue),
        reason: member("reason").as_str().map(str::to_owned),
    }
}

/// JSON whitespace only (RFC 8259: space, tab, line feed, carriage return).
fn is_blank(text: &str) -> bool {
    text.chars()
        .all(|each| matches!(each, ' ' | '\t' | '\n' | '\r'))
}

/// What reading a run gave.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum CoreReading {
    /// A result, or none when the handler expressed no opinion.
    Result {
        /// The result.
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<CoreResult>,
    },
    /// Why it failed.
    Failure {
        /// The cause.
        failure: String,
    },
}

/// J18.22: a timed-out core handler's cause is the timeout text a Claude Code `command` hook gets
/// on the same event: the discarded output named on `UserPromptSubmit`, `timed out` on the others.
fn timeout_text(event: CoreHookEvent, timeout_ms: f64) -> String {
    match event {
        CoreHookEvent::UserPromptSubmit => timed_out_discarded(Some(timeout_ms)),
        CoreHookEvent::SessionStart
        | CoreHookEvent::PreToolUse
        | CoreHookEvent::PostToolUse
        | CoreHookEvent::Stop => TIMED_OUT.to_owned(),
    }
}

/// The failure of a process that did not exit with status 0, worded as a Claude Code exec-form
/// `command` hook's non-blocking error on the same event (J18.22): the launch error as it is, the
/// timeout text, `Failed with non-blocking status code: <first stderr line>` for any other ending
/// (a process ended by a signal has no status and is worded as one, its stderr first).
fn process_failure(event: CoreHookEvent, run: &CommandResult, timeout_ms: f64) -> Option<String> {
    if let Some(start_error) = &run.start_error {
        return Some(start_error.clone());
    }
    if run.timed_out {
        return Some(timeout_text(event, timeout_ms));
    }
    if run.exit_code != Some(0) {
        let line = first_line(&run.stderr);
        let line = if line.is_empty() {
            NO_STDERR
        } else {
            line.as_str()
        };
        return Some(format!("{NON_BLOCKING_STATUS} {line}"));
    }
    None
}

/// Reads one finished run of a core handler for its event.
#[must_use]
pub fn read_core_run(event: CoreHookEvent, run: &CommandResult, timeout_ms: f64) -> CoreReading {
    if let Some(failure) = process_failure(event, run, timeout_ms) {
        return CoreReading::Failure { failure };
    }
    if is_blank(&run.stdout) {
        return CoreReading::Result { result: None };
    }
    let Ok(value) = json_parse(&run.stdout) else {
        return CoreReading::Failure {
            failure: "wrote output that is not one JSON object".to_owned(),
        };
    };
    let JsValue::Object(output) = &value else {
        return not_a_result(event);
    };
    if !shapes(event).iter().any(|shape| matches(shape, output)) {
        return not_a_result(event);
    }
    CoreReading::Result {
        result: Some(result_of(output)),
    }
}

fn not_a_result(event: CoreHookEvent) -> CoreReading {
    CoreReading::Failure {
        failure: format!("wrote an object that is not a {} result", event.as_str()),
    }
}

crate::registry::exports! {
    "hooks/core/output" {
        "readCoreRun" => ok |event: CoreHookEvent, run: CommandResult, timeout_ms: f64| read_core_run(event, &run, timeout_ms);
    }
}
