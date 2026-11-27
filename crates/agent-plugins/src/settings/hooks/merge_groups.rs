//! Port of `src/settings/hooks/merge-groups.ts` at sha256:f4f36b28f2ab.
//!
//! The `hooks` key across settings files (settings-reference.md, `hooks`: "Hooks merge across
//! files rather than replacing each other, and hooks from managed settings can't be removed from
//! other files"; hooks.md, "Hook locations"). Every file's matcher groups are kept, per event, so
//! a file adds hooks and never takes one away. A handler defined in more than one file runs once
//! (hooks.md, "Hook handler fields"): the copy from the lower file is dropped from a group with
//! the same matcher. The same handler under two matchers is kept: which groups match one
//! occurrence is known only when an event fires, so dropping that repeat is the hooks engine's (an
//! open requirement, spec/judgments.md). Only settings files come here; a plugin's or skill's copy
//! stays separate. Running the hooks is the hooks engine's; this only combines what the files
//! define.
//!
//! `isHooksSettings`, a guard over a host value, stays TypeScript in the package (docs/RUST.md,
//! "What stays TypeScript"); it is here for a Rust host and is not an export.

use std::collections::HashMap;
use std::collections::HashSet;

use indexmap::IndexMap;

use crate::format::canonical_json::canonical_json;
use crate::js::{JsObject, JsValue, property_order};

/// A value that has the shape of an event map (an object of arrays); the reader checks the rest.
#[must_use]
pub fn is_hooks_settings(value: &JsValue) -> bool {
    value
        .as_object()
        .is_some_and(|members| members.values().all(JsValue::is_array))
}

/// The matcher as a group key: `"*"`, `""` and an omitted matcher all match every occurrence of
/// the event (hooks.md, "Matcher patterns"), so they are one group. The grammar itself is
/// `rules`'s (`matchesMatcher`), which a module of the same layer cannot import.
fn matcher_key(matcher: Option<&JsValue>) -> String {
    match matcher {
        None | Some(JsValue::Undefined) => String::new(),
        Some(JsValue::String(text)) if text == "*" => String::new(),
        Some(JsValue::String(text)) => text.clone(),
        // A matcher that is no string is its own key, apart from every text.
        Some(other) => format!("\u{0}{}", canonical_json(other)),
    }
}

/// The handlers of the groups already kept for one event, by matcher: what a lower file repeats.
type Seen = HashMap<String, HashSet<String>>;

fn handlers_of(group: &JsValue) -> &[JsValue] {
    group
        .get("hooks")
        .and_then(JsValue::as_array)
        .map_or(&[], Vec::as_slice)
}

fn without_repeats(group: &JsValue, seen: &Seen) -> Option<JsValue> {
    let known = seen.get(&matcher_key(group.get("matcher")));
    let hooks = handlers_of(group);
    let fresh: Vec<JsValue> = hooks
        .iter()
        .filter(|handler| !known.is_some_and(|known| known.contains(&canonical_json(handler))))
        .cloned()
        .collect();
    // A group whose every handler a higher file already runs adds nothing.
    if fresh.is_empty() && !hooks.is_empty() {
        return None;
    }
    let mut copy = group.as_object().cloned().unwrap_or_default();
    copy.insert("hooks".to_owned(), JsValue::Array(fresh));
    Some(JsValue::Object(copy))
}

fn remember(group: &JsValue, seen: &mut Seen) {
    let known = seen.entry(matcher_key(group.get("matcher"))).or_default();
    for handler in handlers_of(group) {
        known.insert(canonical_json(handler));
    }
}

/// Combines the `hooks` of several files, highest precedence first: per event, the groups of the
/// highest file come first, then those of the next, and so on. Within one file nothing is dropped.
#[must_use]
pub fn merge_hook_settings(files: &[JsObject]) -> JsObject {
    let mut merged: IndexMap<String, Vec<JsValue>> = IndexMap::new();
    for file in files {
        let mut added: IndexMap<String, Vec<JsValue>> = IndexMap::new();
        for (event, groups) in property_order(file) {
            let mut seen = Seen::new();
            for group in merged.get(event).into_iter().flatten() {
                remember(group, &mut seen);
            }
            let fresh = groups
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|group| without_repeats(group, &seen))
                .collect();
            added.insert(event.clone(), fresh);
        }
        for (event, fresh) in added {
            merged.entry(event).or_default().extend(fresh);
        }
    }
    merged
        .into_iter()
        .map(|(event, groups)| (event, JsValue::Array(groups)))
        .collect()
}

crate::registry::exports! {
    "settings/hooks/merge-groups" {
        "mergeHookSettings" => ok |files: Vec<JsObject>| merge_hook_settings(&files);
    }
}
