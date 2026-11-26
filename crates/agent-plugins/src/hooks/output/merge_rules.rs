//! Port of `src/hooks/output/merge-rules.ts`, switched.
//!
//! What the per-event merge rules share: one hook's output as the rules read it, and what a rule
//! returns. A rule gets every hook's read, in registry order (the order hooks are registered in, so
//! the result does not depend on which hook finishes first). `Read` and `Merger` are types and stay
//! in the package's TypeScript; the core keeps the values.

use serde::Serialize;

use crate::format::issues::PluginIssue;
use crate::js::{JsObject, JsValue};
use crate::registry::{Rest, Set};

/// What a rule returns, here without a block: `NOTHING` has none.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Merged {
    /// The merged effects.
    pub effects: JsObject,
    /// The issues of the merge.
    pub issues: Vec<PluginIssue>,
}

/// `NOTHING`: the merge of no output at all.
#[must_use]
pub fn nothing() -> Merged {
    Merged {
        effects: JsObject::new(),
        issues: Vec::new(),
    }
}

/// The last defined value: "choose last-in-registry-order wins" (spec/judgments.md). `value ??
/// last`: `null` and `undefined` are not defined.
#[must_use]
pub fn last_of(values: &[JsValue]) -> Option<JsValue> {
    values.iter().fold(None, |last, value| match value {
        JsValue::Undefined | JsValue::Null => last,
        JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Array(_)
        | JsValue::Object(_) => Some(value.clone()),
    })
}

/// `new Set(events)`: each event once, in the order first given.
#[must_use]
pub fn event_set(events: Vec<String>) -> Set<String> {
    let mut members: Vec<String> = Vec::new();
    for event in events {
        if !members.contains(&event) {
            members.push(event);
        }
    }
    Set { members }
}

crate::registry::exports! {
    "hooks/output/merge-rules" {
        "NOTHING" => ok nothing;
        "lastOf" => ok |values: Vec<JsValue>| last_of(&values);
        "eventSet" => ok |events: Rest<String>| event_set(events.0);
    }
}
