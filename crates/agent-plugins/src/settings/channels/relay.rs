//! Port of `src/settings/channels/relay.ts`, switched.
//!
//! Permission relay's request side (channels-reference.md, "Relay permission prompts"): the IDs
//! Claude Code issues, the four fields of a request, and the rule that applies a verdict only when
//! its ID matches an open request.
//!
//! Across the boundary the relay is a handle whose methods the host calls; `open` asks the host's
//! `random` as it goes, and stays sync for the host.

use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use super::sanitize::{input_preview, relay_description};
use crate::js::JsObject;
use crate::ports::BoxFuture;
use crate::registry::{Callback, Handle};

/// "Five lowercase letters drawn from `a`-`z` without `l`."
const REQUEST_ID_ALPHABET: &str = "abcdefghijkmnopqrstuvwxyz";
const REQUEST_ID_LENGTH: usize = 5;
/// `REQUEST_ID_ALPHABET.length`.
const ALPHABET_LENGTH: f64 = 25.0;

/// The `notifications/claude/channel/permission_request` params.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PermissionRequestParams {
    /// The request's ID.
    pub request_id: String,
    /// The tool asked about.
    pub tool_name: String,
    /// What the call does.
    pub description: String,
    /// The input, shortened.
    pub input_preview: String,
}

/// A verdict's behaviour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Behavior {
    /// Allow the call.
    Allow,
    /// Deny it.
    Deny,
}

/// The `notifications/claude/channel/permission` params a channel sends back.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PermissionVerdictParams {
    /// The request it answers.
    pub request_id: String,
    /// The answer.
    pub behavior: Behavior,
}

/// The call a permission prompt is about.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayToolCall {
    /// The tool.
    pub tool_name: String,
    /// What the call does.
    #[serde(default)]
    pub description: Option<String>,
    /// The call's input.
    pub input: JsObject,
}

/// Whether a verdict was applied, and with what.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum RelayOutcome {
    /// Applied.
    Applied {
        /// Always `true`.
        applied: bool,
        /// The verdict's behaviour.
        behavior: Behavior,
    },
    /// Dropped.
    Dropped {
        /// Always `false`.
        applied: bool,
    },
}

/// The host's source of randomness (L3): a number in [0, 1). The host's function, so its answer
/// may wait; a Rust host passes a closure, answered at once.
pub trait Random: Send + Sync + 'static {
    /// The next number.
    fn random(&self) -> BoxFuture<'_, f64>;
}

impl<F: Fn() -> f64 + Send + Sync + 'static> Random for F {
    fn random(&self) -> BoxFuture<'_, f64> {
        Box::pin(std::future::ready(self()))
    }
}

impl Random for Callback {
    fn random(&self) -> BoxFuture<'_, f64> {
        Box::pin(self.call(Vec::new()))
    }
}

#[derive(Default)]
struct Ids {
    open: HashSet<String>,
    issued: HashSet<String>,
    repeated: HashSet<String>,
}

/// The request side of the relay.
pub struct PermissionRelay<R> {
    random: R,
    ids: Mutex<Ids>,
}

impl<R: Random> PermissionRelay<R> {
    /// `REQUEST_ID_LENGTH` letters, each `REQUEST_ID_ALPHABET[Math.floor(random() * length)]`
    /// (an index outside the alphabet adds nothing, as `join` writes `undefined`).
    async fn new_id(&self) -> String {
        let letters: Vec<char> = REQUEST_ID_ALPHABET.chars().collect();
        let mut id = String::new();
        for _ in 0..REQUEST_ID_LENGTH {
            let at = (self.random.random().await * ALPHABET_LENGTH).floor();
            if (0.0..ALPHABET_LENGTH).contains(&at)
                && let Some(letter) = letters.get(crate::js::slice_index(at, letters.len()))
            {
                id.push(*letter);
            }
        }
        id
    }

    fn ids(&self) -> std::sync::MutexGuard<'_, Ids> {
        self.ids.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Opens a request for a permission prompt and returns what to notify the channel with.
    pub async fn open(&self, call: &RelayToolCall) -> PermissionRequestParams {
        let id = self.new_id().await;
        let mut ids = self.ids();
        // "Once an ID repeats, Claude Code drops any channel verdict carrying it."
        if ids.issued.contains(&id) {
            ids.repeated.insert(id.clone());
        }
        ids.issued.insert(id.clone());
        ids.open.insert(id.clone());
        PermissionRequestParams {
            request_id: id,
            tool_name: call.tool_name.clone(),
            description: relay_description(call.description.as_deref()),
            input_preview: input_preview(&call.input),
        }
    }

    /// A verdict from a channel: applied "only if the ID matches an open request".
    pub fn verdict(&self, params: &PermissionVerdictParams) -> RelayOutcome {
        let mut ids = self.ids();
        let id = &params.request_id;
        if !ids.open.contains(id) || ids.repeated.contains(id) {
            return RelayOutcome::Dropped { applied: false };
        }
        ids.open.remove(id);
        RelayOutcome::Applied {
            applied: true,
            behavior: params.behavior,
        }
    }

    /// Someone answered at the terminal: "that answer is applied instead and the pending remote
    /// request is dropped".
    pub fn answer_locally(&self, request_id: &str) {
        self.ids().open.remove(request_id);
    }
}

/// `random` returns a number in [0, 1), the host's source of randomness (L3).
pub fn create_permission_relay<R: Random>(random: R) -> PermissionRelay<R> {
    PermissionRelay {
        random,
        ids: Mutex::new(Ids::default()),
    }
}

crate::registry::methods! {
    PermissionRelay<Callback> {
        "open" => sync async ok |this, call: RelayToolCall| this.open(&call).await;
        "verdict" => ok |this, params: PermissionVerdictParams| this.verdict(&params);
        "answerLocally" => ok |this, request_id: String| this.answer_locally(&request_id);
    }
}

crate::registry::exports! {
    "settings/channels/relay" {
        "createPermissionRelay" => ok |random: Callback| Handle::new(create_permission_relay(random));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::now;

    #[test]
    fn a_verdict_applies_once_to_an_open_id() {
        let relay = create_permission_relay(|| 0.0);
        let call = RelayToolCall {
            tool_name: "Bash".to_owned(),
            description: None,
            input: JsObject::new(),
        };
        let opened = now(relay.open(&call)).map(|params| params.request_id);
        assert_eq!(opened.as_deref(), Some("aaaaa"));
        let verdict = PermissionVerdictParams {
            request_id: "aaaaa".to_owned(),
            behavior: Behavior::Allow,
        };
        assert_eq!(
            relay.verdict(&verdict),
            RelayOutcome::Applied {
                applied: true,
                behavior: Behavior::Allow
            }
        );
        assert_eq!(
            relay.verdict(&verdict),
            RelayOutcome::Dropped { applied: false }
        );
    }
}
