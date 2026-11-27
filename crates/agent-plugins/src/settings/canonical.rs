//! Port of `src/settings/canonical.ts`, switched.
//!
//! The lists as one, each entry once, first occurrence kept, so the highest file's entries come
//! first: two entries are one when their canonical text is the same (managed-settings.md, "Split a
//! file-based policy across teams": the lists combine "with duplicates removed"). The one union of
//! the settings layer.

use std::collections::HashSet;

use crate::format::canonical_json::canonical_json;
use crate::js::JsValue;

/// The union of `lists`, each entry once, the first occurrence kept.
#[must_use]
pub fn union_of(lists: &[Vec<JsValue>]) -> Vec<JsValue> {
    let mut seen: HashSet<String> = HashSet::new();
    lists
        .iter()
        .flatten()
        .filter(|entry| seen.insert(canonical_json(entry)))
        .cloned()
        .collect()
}

crate::registry::exports! {
    "settings/canonical" {
        "unionOf" => ok |lists: Vec<Vec<JsValue>>| union_of(&lists);
    }
}
