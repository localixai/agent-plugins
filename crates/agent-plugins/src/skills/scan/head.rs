//! Ported from `src/skills/scan/head.ts`; only the core uses it now, so it has no wrapper.
//!
//! The skill or command without its body, field by field.

use crate::js::JsObject;

/// The skill or command without its body (a skill is the one with a `dir`; both lose `body`).
#[must_use]
pub fn head_of(mut invocable: JsObject) -> JsObject {
    invocable.shift_remove("body");
    invocable
}
