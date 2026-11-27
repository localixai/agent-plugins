//! Port of `src/hooks/select/dedupe.ts`, switched.
//!
//! Its answer holds copies of the entries; the selection (`select.rs`) keeps each entry's place
//! instead, so the host gets its own entries back.

use std::borrow::Borrow;

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

/// The entries with each settings handler kept once, the first in registry order. An item is an
/// entry, or anything that holds one (an entry with its place in the registry).
#[must_use]
pub fn dedupe_settings_handlers<T: Borrow<JsValue>>(entries: Vec<T>) -> Vec<T> {
    let mut seen = IndexSet::new();
    entries
        .into_iter()
        .filter(|item| {
            let entry = item.borrow();
            !merges_copies(entry) || seen.insert(identity(entry))
        })
        .collect()
}

crate::registry::exports! {
    "hooks/select/dedupe" {
        "dedupeSettingsHandlers" => ok |entries: Vec<JsValue>| dedupe_settings_handlers(entries);
    }
}
