//! Port of `src/rules/decide/no-approval-surface.ts`, switched.
//!
//! The messages of `permissions.denyInsteadOfPrompting` (settings-reference.md, "Messages from
//! `denyInsteadOfPrompting`"), verbatim: they report refused calls and nothing waits on them. The
//! host writes them where the docs put them; the kit holds the texts and the count of refusals.

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::registry::Handle;

/// "The denial Claude reads can open with this text."
pub const NO_APPROVAL_SURFACE_DENIAL: &str = "Permission for this tool use was denied. It requires approval, and this session has no approval surface";

/// The `decision_reason` of a `permission_denied` message in `stream-json` output.
pub const NO_APPROVAL_SURFACE_DECISION_REASON: &str =
    "no approval surface in this session; permission request denied automatically";

/// "The turn ends after 20 calls in a row that the classifier blocked or a safety check refused".
pub const REFUSAL_STOP_LIMIT: f64 = 20.0;

/// What the person sees when those refusals end the turn.
pub const REFUSAL_STOP_MESSAGE: &str =
    "20 actions were refused and no work went ahead in between, so Claude stopped.";

/// "A `-p` run reports that turn as `error_during_execution`."
pub const REFUSAL_STOP_RESULT: &str = "error_during_execution";

/// One call of an auto mode turn: refused by the classifier or a safety check, or work that went ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RefusalEvent {
    /// `refused`.
    Refused,
    /// `went-ahead`.
    WentAhead,
}

/// Where the turn stands after a call: it goes on, or it ends with the message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum RefusalVerdict {
    /// The turn goes on.
    Continue,
    /// The turn ends.
    Stop {
        /// What the person sees.
        message: String,
    },
}

/// The count of refusals in a row of one turn.
#[derive(Debug, Default)]
pub struct RefusalStreak {
    count: Mutex<f64>,
}

/// How each call moves the count: a refusal adds one, work that went ahead starts it over.
const fn step(event: RefusalEvent, count: f64) -> f64 {
    match event {
        RefusalEvent::Refused => count + 1.0,
        RefusalEvent::WentAhead => 0.0,
    }
}

impl RefusalStreak {
    /// Records a call and says whether the turn ends.
    pub fn record(&self, event: RefusalEvent) -> RefusalVerdict {
        let mut count = self.count.lock().unwrap_or_else(PoisonError::into_inner);
        *count = step(event, *count);
        if *count >= REFUSAL_STOP_LIMIT {
            RefusalVerdict::Stop {
                message: REFUSAL_STOP_MESSAGE.to_owned(),
            }
        } else {
            RefusalVerdict::Continue
        }
    }
}

crate::registry::methods! {
    RefusalStreak {
        "record" => ok |this, event: RefusalEvent| this.record(event);
    }
}

/// The count of one turn "in auto mode with the setting on": 20 refusals in a row, with "no work
/// went ahead in between", end it. A host makes one per turn, and only in that case.
#[must_use]
pub fn create_refusal_streak() -> RefusalStreak {
    RefusalStreak::default()
}

crate::registry::exports! {
    "rules/decide/no-approval-surface" {
        "NO_APPROVAL_SURFACE_DENIAL" => ok || NO_APPROVAL_SURFACE_DENIAL;
        "NO_APPROVAL_SURFACE_DECISION_REASON" => ok || NO_APPROVAL_SURFACE_DECISION_REASON;
        "REFUSAL_STOP_LIMIT" => ok || REFUSAL_STOP_LIMIT;
        "REFUSAL_STOP_MESSAGE" => ok || REFUSAL_STOP_MESSAGE;
        "REFUSAL_STOP_RESULT" => ok || REFUSAL_STOP_RESULT;
        "createRefusalStreak" => ok || Handle::new(create_refusal_streak());
    }
}
