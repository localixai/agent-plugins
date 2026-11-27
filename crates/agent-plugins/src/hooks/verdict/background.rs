//! Port of `src/hooks/verdict/background.ts`, switched.
//!
//! What a background hook hands over once its process ends (hooks.md, "How async hooks execute"):
//! "Claude Code delivers the `additionalContext` and `systemMessage` fields from the hook's JSON
//! response to Claude on the next conversation turn". The response is validated and "any field whose
//! value has the wrong type, such as a `systemMessage` that isn't a string" is dropped. Nothing else
//! of the response matters: "response fields like `decision`, `permissionDecision`, and `continue`
//! have no effect, because the action they would have controlled has already completed". An
//! `asyncRewake` hook that exited with code 2 "wakes Claude" with its stderr or stdout.

use super::stdout_kind::read_async_response;
use super::types::{BackgroundDelivery, BackgroundOutcome};
use crate::format::issues::{PluginIssue, warning};
use crate::hooks::handlers::context::{BackgroundResult, ForegroundRun};
use crate::js::JsValue;
use crate::protocol::is_record::is_record;

fn text_of(value: &JsValue) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// "drops each field of the wrong type instead of delivering it, with a debug warning naming the
/// field" (Agent Plugins Extended §7.3.3 item 3): a delivered field that is present and not a string.
fn dropped_field(handler_id: &str, field: &str, value: &JsValue) -> Vec<PluginIssue> {
    if matches!(value, JsValue::Undefined | JsValue::String(_)) {
        Vec::new()
    } else {
        vec![warning(
            &format!("hooks.{handler_id}.{field}"),
            &format!("The async hook response field {field} is not a string and is dropped"),
        )]
    }
}

/// How each kind of run ended.
const fn outcome_of(run: &ForegroundRun) -> BackgroundOutcome {
    match run {
        ForegroundRun::Exited(_) => BackgroundOutcome::Completed,
        ForegroundRun::TimedOut => BackgroundOutcome::TimedOut,
        ForegroundRun::NotStarted { .. } => BackgroundOutcome::Failed,
        ForegroundRun::Cancelled => BackgroundOutcome::Cancelled,
    }
}

/// The response member `key`, `undefined` when there is none.
fn member(value: &JsValue, key: &str) -> JsValue {
    if is_record(value) {
        value.get(key).cloned().unwrap_or_default()
    } else {
        JsValue::Undefined
    }
}

/// What a background hook hands over once its process ends.
#[must_use]
pub fn delivery_of(handler_id: &str, result: &BackgroundResult) -> BackgroundDelivery {
    let response = match &result.run {
        ForegroundRun::Exited(exited) => read_async_response(&exited.stdout),
        ForegroundRun::TimedOut | ForegroundRun::NotStarted { .. } | ForegroundRun::Cancelled => {
            None
        }
    };
    let specific = response.as_ref().map_or(JsValue::Undefined, |response| {
        member(response, "hookSpecificOutput")
    });
    let given_context = member(&specific, "additionalContext");
    let given_message = response.as_ref().map_or(JsValue::Undefined, |response| {
        member(response, "systemMessage")
    });
    let additional_context = text_of(&given_context);
    let system_message = text_of(&given_message);
    let mut issues = dropped_field(
        handler_id,
        "hookSpecificOutput.additionalContext",
        &given_context,
    );
    issues.extend(dropped_field(handler_id, "systemMessage", &given_message));
    BackgroundDelivery {
        handler_id: handler_id.to_owned(),
        outcome: outcome_of(&result.run),
        exit_code: match &result.run {
            ForegroundRun::Exited(exited) => Some(exited.exit_code),
            ForegroundRun::TimedOut
            | ForegroundRun::NotStarted { .. }
            | ForegroundRun::Cancelled => None,
        },
        additional_context,
        system_message,
        wake: result.wake.clone(),
        issues: if issues.is_empty() {
            None
        } else {
            Some(issues)
        },
    }
}

crate::registry::exports! {
    "hooks/verdict/background" {
        "deliveryOf" => ok |handler_id: String, result: BackgroundResult| delivery_of(&handler_id, &result);
    }
}
