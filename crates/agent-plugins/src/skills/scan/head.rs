//! The core's counterpart of `src/skills/scan/head.ts` at sha256:87210c378788, which stays TypeScript: it
//! only looks at the outside of a host value (whether it has a `dir`) and drops its `body`, which
//! crossing the boundary would copy there and back (docs/RUST.md, "What stays TypeScript").
//!
//! The skill or command without its body, field by field.

use crate::js::JsObject;

/// The skill or command without its body (a skill is the one with a `dir`; both lose `body`).
#[must_use]
pub fn head_of(mut invocable: JsObject) -> JsObject {
    invocable.shift_remove("body");
    invocable
}
