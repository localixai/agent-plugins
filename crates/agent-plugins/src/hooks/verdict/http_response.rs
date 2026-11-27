//! Ported from `src/hooks/verdict/http-response.ts`; only the core uses it now, so it has no wrapper.
//!
//! HTTP response handling (hooks.md, "HTTP response handling"). A hook answers with a status and a
//! body instead of an exit code and stdout, and never blocks by itself ("To block the action when
//! the request fails or returns a non-2xx status, set `onFailure: "block"`"):
//!
//! - 2xx with an empty body: success, "equivalent to exit code 0 with no output"
//! - 2xx with a JSON object body: parsed with the schema of command hooks; a body that fails
//!   validation is a non-blocking error
//! - 2xx with any other body: a non-blocking error, "handled the same as a non-2xx status"
//! - non-2xx status and connection failure: a non-blocking error
//! - timeout: the shared `timed-out` run

use super::debug_lines::{Subject, unrecognized_keys_note, with_notes};
use super::on_failure::{Failure, FailureCause, failure_verdict};
use super::stdout_kind::read_body;
use crate::js::{JsObject, JsValue, number_to_string, trim};

fn is_success_status(status: f64) -> bool {
    (200.0..300.0).contains(&status)
}

fn success(subject: &Subject, output: Option<JsObject>) -> JsObject {
    let mut verdict = JsObject::from_iter([
        ("handler".to_owned(), subject.entry.clone()),
        ("status".to_owned(), JsValue::from("success")),
        ("succeeded".to_owned(), JsValue::Bool(true)),
        ("issues".to_owned(), JsValue::Array(vec![])),
    ]);
    if let Some(output) = output {
        verdict.insert("output".to_owned(), JsValue::Object(output));
    }
    if subject.effects_ignored {
        verdict.insert("effectsIgnored".to_owned(), JsValue::Bool(true));
    }
    verdict
}

fn invalid(text: String) -> Failure {
    Failure {
        cause: FailureCause::InvalidOutput,
        detail: text.clone(),
        text,
    }
}

async fn interpret_body(subject: &Subject, body: &str) -> JsObject {
    let reading = read_body(body);
    if reading.empty {
        return success(subject, None);
    }
    let Some(json) = reading.json else {
        let text = format!(
            "the response body is neither empty nor a JSON object: {}",
            reading.not_object.unwrap_or_default()
        );
        return failure_verdict(subject, &invalid(trim(&text).to_owned()));
    };
    let checked = subject.parse(&subject.event, json).await;
    match checked.output {
        None => failure_verdict(
            subject,
            &invalid(
                checked
                    .failure
                    .unwrap_or_else(|| "the response failed validation".to_owned()),
            ),
        ),
        Some(output) => with_notes(
            success(subject, Some(output)),
            &unrecognized_keys_note(subject, checked.unrecognized_keys.as_deref()),
        ),
    }
}

/// What an HTTP hook's response comes to.
pub async fn interpret_http(subject: &Subject, status: f64, body: &str) -> JsObject {
    if is_success_status(status) {
        return interpret_body(subject, body).await;
    }
    let text = format!("HTTP {}", number_to_string(status));
    failure_verdict(
        subject,
        &Failure {
            cause: FailureCause::HttpError,
            detail: text.clone(),
            text,
        },
    )
}

/// What an HTTP hook whose request failed comes to.
#[must_use]
pub fn interpret_http_failed(subject: &Subject, message: &str) -> JsObject {
    failure_verdict(
        subject,
        &Failure {
            cause: FailureCause::HttpError,
            text: message.to_owned(),
            detail: message.to_owned(),
        },
    )
}
