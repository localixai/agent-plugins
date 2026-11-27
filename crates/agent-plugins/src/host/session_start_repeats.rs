//! Port of `src/host/session-start-repeats.ts`, switched.
//!
//! The repeat check of async `SessionStart` hooks (hooks.md, "Limitations"; Agent Plugins Extended
//! §7.3.3 item 7): the hook runs every time the event fires, and its output is skipped when its
//! `additionalContext` and `systemMessage` equal what Claude last received from that hook, unless a
//! synchronous `SessionStart` hook added text since. A compaction or `/clear` forgets what each
//! hook gave, so the first run after it delivers again.
//!
//! Judgment (spec/judgments.md): "what Claude last received" is the last delivery of that handler
//! the session handed on, and any text a synchronous `SessionStart` hook adds counts as "other
//! text".
//!
//! The outcome's background deliveries are the host's promises: another event's go back as given,
//! and a `SessionStart`'s become promises of the core's that settle with the delivery passed
//! through the check (`promise.then(pass)`), a rejection kept.

use std::sync::{Arc, Mutex, PoisonError};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::json_stringify::json_stringify;
use crate::js::{JsObject, JsValue};
use crate::registry::{CorePromise, Handle, HostPromise};

/// The `SessionStart` sources after which every hook delivers again.
const FORGETS: [&str; 2] = ["compact", "clear"];

#[derive(Debug, Clone, PartialEq)]
struct Given {
    text: String,
    /// The count of synchronous additions when it was given.
    added: u64,
}

fn text_of(delivery: &JsObject) -> String {
    let member = |key: &str| match delivery.get(key) {
        None | Some(JsValue::Undefined | JsValue::Null) => JsValue::String(String::new()),
        Some(value) => value.clone(),
    };
    json_stringify(&JsValue::Array(vec![
        member("additionalContext"),
        member("systemMessage"),
    ]))
    .unwrap_or_default()
}

fn without_text(mut delivery: JsObject) -> JsObject {
    delivery.shift_remove("additionalContext");
    delivery.shift_remove("systemMessage");
    delivery
}

#[derive(Debug, Default)]
struct Repeats {
    given: IndexMap<String, Given>,
    added: u64,
}

fn is_absent(delivery: &JsObject, key: &str) -> bool {
    delivery.get(key).is_none_or(JsValue::is_undefined)
}

fn pass(state: &Mutex<Repeats>, delivery: JsObject) -> JsObject {
    if is_absent(&delivery, "additionalContext") && is_absent(&delivery, "systemMessage") {
        return delivery;
    }
    let text = text_of(&delivery);
    let handler_id = delivery
        .get("handlerId")
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut repeats = state.lock().unwrap_or_else(PoisonError::into_inner);
    let added = repeats.added;
    if repeats
        .given
        .get(&handler_id)
        .is_some_and(|last| last.text == text && last.added == added)
    {
        return without_text(delivery);
    }
    repeats.given.insert(handler_id, Given { text, added });
    delivery
}

/// The part of the outcome the check reads.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedOutcome {
    /// The event.
    pub event: String,
    /// The background deliveries, the host's promises.
    #[serde(default)]
    pub background: Vec<HostPromise<JsObject>>,
    /// The context the synchronous hooks added.
    #[serde(default)]
    pub additional_context: Vec<JsValue>,
    /// The messages the synchronous hooks added.
    #[serde(default)]
    pub system_messages: Vec<JsValue>,
}

/// A background promise as the check hands it on.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Watched {
    /// The host's promise, as given.
    Given(HostPromise<JsObject>),
    /// The delivery passed through the check.
    Passed(CorePromise<JsObject>),
}

/// The repeat check of one session.
#[derive(Debug, Default)]
pub struct SessionStartRepeats {
    state: Arc<Mutex<Repeats>>,
}

impl SessionStartRepeats {
    /// The background promises of one dispatch, with the repeats of a `SessionStart`'s emptied.
    pub fn watch(&self, outcome: WatchedOutcome, source: &JsValue) -> Vec<Watched> {
        if outcome.event != "SessionStart" {
            return outcome.background.into_iter().map(Watched::Given).collect();
        }
        {
            let mut repeats = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if source
                .as_str()
                .is_some_and(|source| FORGETS.contains(&source))
            {
                repeats.given.clear();
            }
            if outcome.additional_context.len() + outcome.system_messages.len() > 0 {
                repeats.added += 1;
            }
        }
        outcome
            .background
            .into_iter()
            .map(|promise| {
                let state = Arc::clone(&self.state);
                Watched::Passed(CorePromise::new(async move { pass(&state, promise.await) }))
            })
            .collect()
    }
}

/// A check of its own.
#[must_use]
pub fn create_session_start_repeats() -> SessionStartRepeats {
    SessionStartRepeats::default()
}

crate::registry::methods! {
    SessionStartRepeats {
        "watch" => ok |this, outcome: WatchedOutcome, source: JsValue| this.watch(outcome, &source);
    }
}

crate::registry::exports! {
    "host/session-start-repeats" {
        "createSessionStartRepeats" => ok || Handle::new(create_session_start_repeats());
    }
}
