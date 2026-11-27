//! Ported from `src/hooks/handlers/status.ts`; only the core uses it now, so it has no wrapper; the
//! TypeScript keeps the shapes the host's observer is called with.
//!
//! `statusMessage` (hooks.md, "Common fields"): "Custom spinner message displayed while the hook
//! runs". Showing a spinner is the host's job; the kit tells it when a hook starts and with what
//! text.

use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::registry::{Callback, Port, catch_host};

/// What the host is told when a hook starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookStarted {
    /// The event.
    pub event: String,
    /// The handler's id.
    pub handler_id: String,
    /// Absent when the handler has none: the host shows its own label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
}

/// The part of the host's hook observer the run stage drives: the host's object when it holds a
/// function (`started`), else data, which has no callback to call.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum StartObserver {
    /// The observer holds a function.
    Host(Port),
    /// The observer is data.
    Data(JsValue),
}

/// The handler's own fields a start names.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusFields {
    /// The spinner text.
    #[serde(default)]
    pub status_message: Option<String>,
}

/// The part of a registered handler a start names.
#[derive(Clone, Debug, Deserialize)]
pub struct StartedEntry {
    /// The handler's id.
    pub id: String,
    /// The event.
    pub event: String,
    /// The handler.
    #[serde(default)]
    pub handler: StatusFields,
}

/// Calls one of the host's observer callbacks. A callback that throws is the host's fault and tells
/// the host nothing it can use mid-dispatch: it neither stops the hook nor rejects the dispatch, so
/// a sibling hook's block still reaches the outcome (judgment, spec/judgments.md).
pub async fn observe<F: Future>(call: F) {
    // The observer only watches: what it throws is dropped.
    let _ = catch_host(call).await;
}

/// Tells the observer that `entry` starts.
pub async fn announce_start(observer: Option<&StartObserver>, entry: &StartedEntry) {
    let Some(StartObserver::Host(observer)) = observer else {
        return;
    };
    let info = HookStarted {
        event: entry.event.clone(),
        handler_id: entry.id.clone(),
        status_message: entry.handler.status_message.clone(),
    };
    let args = vec![to_js_value(&info).unwrap_or_default()];
    let observer = *observer;
    observe(async move {
        if observer
            .member::<Option<Callback>>("started")
            .await
            .is_some()
        {
            let _: JsValue = observer.method("started", args).await;
        }
    })
    .await;
}
