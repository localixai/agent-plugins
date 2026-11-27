//! Ported from `src/hooks/handlers/remote/http.ts`; only the core uses it now, so it has no wrapper.
//!
//! An `http` hook (hooks.md, "HTTP hook fields"): "Claude Code sends the hook's JSON input as the
//! POST request body with `Content-Type: application/json`". The run is the raw answer; what a
//! status or a body means ("HTTP response handling") is the interpreter's.

use indexmap::IndexMap;
use serde::Serialize;

use super::failure_run::{EndedRun, EntryEvent, SkippedRun, ended_run, remote_path, without_port};
use super::http_env::interpolate_headers;
use super::http_policy::{blocked_url_issue, permitted_env_vars, url_allowed};
use crate::format::issues::PluginIssue;
use crate::hooks::handlers::context::{RunContext, RunContextRead};
use crate::hooks::handlers::timeouts::{TimedEntry, handler_timeout_ms};
use crate::js::{JsObject, JsValue};
use crate::ports::http_client::{BodyAs, HttpClient, HttpMethod, HttpRequest, Redirect};

/// `{"content-type": "application/json"}`.
const JSON_TYPE: (&str, &str) = ("content-type", "application/json");

/// What an `http` run was: not run, ended by a timeout or an abort, a failure, or an answer.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum HttpRun {
    /// It did not run.
    Skipped(SkippedRun),
    /// `timed-out` or `cancelled`.
    Ended(EndedRun),
    /// The request failed.
    Failed(HttpFailed),
    /// The server answered.
    Answer(HttpAnswer),
}

/// `{ kind: 'http-failed', message }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HttpFailed {
    /// `http-failed`.
    pub kind: &'static str,
    /// What the host saw.
    pub message: String,
}

/// `{ kind: 'http', status, body }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HttpAnswer {
    /// `http`.
    pub kind: &'static str,
    /// The status code.
    pub status: u16,
    /// The body.
    pub body: String,
}

/// The raw run and the issues the run itself found.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HttpHandled {
    /// The run.
    pub run: HttpRun,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// `value` as text, `""` when it is none.
fn text_of(handler: &JsObject, key: &str) -> String {
    handler
        .get(key)
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The handler's `headers`, a record of strings.
fn headers_of(handler: &JsObject) -> Option<IndexMap<String, String>> {
    let JsValue::Object(members) = handler.get("headers")? else {
        return None;
    };
    Some(
        members
            .iter()
            .map(|(name, value)| (name.clone(), value.as_str().unwrap_or_default().to_owned()))
            .collect(),
    )
}

/// The handler's `allowedEnvVars`, a list of strings.
fn allowed_env_vars_of(handler: &JsObject) -> Option<Vec<String>> {
    let JsValue::Array(items) = handler.get("allowedEnvVars")? else {
        return None;
    };
    Some(
        items
            .iter()
            .map(|item| item.as_str().unwrap_or_default().to_owned())
            .collect(),
    )
}

/// A limit in milliseconds as the port takes it: whole.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a limit is a positive whole number of milliseconds (`handler_timeout_ms` rounds it)"
)]
pub(super) const fn whole_ms(ms: f64) -> u64 {
    ms as u64
}

/// Runs one `http` hook. `entry` is the registered handler, `context` the run's `RunContext`.
pub async fn run_http(entry: &TimedEntry, context: RunContext) -> HttpHandled {
    let RunContextRead {
        ports,
        input,
        timeouts,
        http_policy,
        signal,
        ..
    } = context.read().await;
    let Some(client) = ports.http else {
        let without = without_port(
            &EntryEvent {
                event: entry.event.clone(),
            },
            "http",
            "HttpClient",
        );
        return HttpHandled {
            run: HttpRun::Skipped(without.run),
            issues: without.issues,
        };
    };
    let handler = &entry.handler;
    let url = text_of(handler, "url");
    let allowed_urls = http_policy
        .as_ref()
        .and_then(|policy| policy.allowed_urls.as_deref());
    if !url_allowed(&url, allowed_urls) {
        let event = EntryEvent {
            event: entry.event.clone(),
        };
        let issue = blocked_url_issue(&remote_path(&event, "url"), &url);
        return HttpHandled {
            run: HttpRun::Skipped(SkippedRun {
                kind: "skipped",
                reason: issue.message.clone(),
            }),
            issues: vec![issue],
        };
    }
    let timeout_ms = handler_timeout_ms(entry, &timeouts);
    let permitted = permitted_env_vars(allowed_env_vars_of(handler), http_policy.as_ref());
    let mut headers = interpolate_headers(
        headers_of(handler).as_ref(),
        permitted.as_deref(),
        &ports.environment,
    )
    .await;
    // Judgment: the kit's Content-Type wins over one a config sets.
    headers.insert(JSON_TYPE.0.to_owned(), JSON_TYPE.1.to_owned());
    let answer = client
        .request(HttpRequest {
            method: HttpMethod::Post,
            url,
            headers,
            body: Some(input),
            body_as: BodyAs::Text,
            timeout_ms: timeout_ms.map(whole_ms),
            max_bytes: None,
            redirect: Redirect::Follow,
            signal,
        })
        .await;
    let run = match answer {
        Err(failure) => ended_run(&failure).map_or_else(
            || {
                HttpRun::Failed(HttpFailed {
                    kind: "http-failed",
                    message: failure.message.clone(),
                })
            },
            HttpRun::Ended,
        ),
        Ok(response) => HttpRun::Answer(HttpAnswer {
            kind: "http",
            status: response.status,
            body: response.body,
        }),
    };
    HttpHandled {
        run,
        issues: Vec::new(),
    }
}
