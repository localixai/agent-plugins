//! The core's counterpart of `src/protocol/is-record.ts` at sha256:479eb63580cc, which stays TypeScript: a
//! guard over a host value, which crossing the boundary would copy whole (docs/RUST.md, "What stays TypeScript").
//! A plain JSON-style object: not `null`, not an array. The one guard every reader of parsed
//! JSON, YAML or a host value uses before it reads keys.

use crate::js::JsValue;

/// Whether `value` is an object, the only value that has keys to read.
#[must_use]
pub fn is_record(value: &JsValue) -> bool {
    value.is_object()
}
