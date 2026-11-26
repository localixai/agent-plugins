//! The core's counterpart of `src/hooks/select/dedupe.ts` at sha256:fac6be892490, which stays TypeScript:
//! its result is the host's own entries, which the selection that stays TypeScript tells apart by
//! identity (`indexOf`, a `Map` keyed by entry), and a copy is not that object (docs/RUST.md,
//! "What stays TypeScript").

use indexmap::IndexSet;

use crate::format::canonical_json::canonical_json;
use crate::hooks::contracts::RegisteredHandler;
use crate::hooks::source_kind::source_policy;
use crate::js::JsValue;

/// Judgment (spec/judgments.md): identical means the same event, matcher text and handler fields.
fn identity(entry: &JsValue) -> String {
    let member = |key: &str| entry.get(key).cloned().unwrap_or(JsValue::Undefined);
    let given = member("matcher");
    let matcher = if given.is_nullish() {
        JsValue::String(String::new())
    } else {
        given
    };
    canonical_json(&JsValue::Array(vec![
        member("event"),
        matcher,
        member("handler"),
    ]))
}

/// The registration a whole entry holds, as the gates and policies read it.
#[must_use]
pub fn registered(entry: &JsValue) -> Option<RegisteredHandler> {
    serde::Deserialize::deserialize(entry.clone()).ok()
}

fn merges_copies(entry: &JsValue) -> bool {
    let Some(kind) = entry
        .get("source")
        .and_then(|source| source.get("kind"))
        .and_then(JsValue::as_str)
    else {
        return false;
    };
    source_policy()
        .get(kind)
        .is_some_and(|policy| policy.merges_copies)
}

/// The entries with each settings handler kept once, the first in registry order.
#[must_use]
pub fn dedupe_settings_handlers(entries: Vec<JsValue>) -> Vec<JsValue> {
    let mut seen = IndexSet::new();
    entries
        .into_iter()
        .filter(|entry| !merges_copies(entry) || seen.insert(identity(entry)))
        .collect()
}
