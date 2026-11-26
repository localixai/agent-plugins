//! The core's counterpart of `src/hooks/input/fact-read.ts` at sha256:069efd08544f, which stays TypeScript:
//! a guard over a host value, which crossing the boundary would copy whole (docs/RUST.md, "What
//! stays TypeScript").
//!
//! Reads one field of an event's facts, for the rules that depend on one (`source`, `tool_name`,
//! `prompt`, `agent_id`). Only the facts' own fields count, never one inherited through the
//! prototype.

use crate::js::JsValue;

/// The fact `key`, `undefined` when the facts do not have it.
#[must_use]
pub fn fact_of(facts: &JsValue, key: &str) -> JsValue {
    facts.get(key).cloned().unwrap_or(JsValue::Undefined)
}

/// A fact that is a string; any other value reads as absent.
#[must_use]
pub fn text_fact_of(facts: &JsValue, key: &str) -> Option<String> {
    facts.get(key).and_then(JsValue::as_str).map(str::to_owned)
}
