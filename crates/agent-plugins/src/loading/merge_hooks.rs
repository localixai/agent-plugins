//! Port of `src/loading/merge-hooks.ts`, switched.
//!
//! Joins the hooks of a plugin's sources: "Claude Code merges whatever you declare with
//! `hooks/hooks.json` when that file exists" and "Hooks in `hooks/hooks.json` and in the `hooks`
//! manifest key both load" (manifest-reference.md, "`hooks`"; components.md, "Hooks"). Judgment
//! (spec/judgments.md): merging is per event, the matcher groups of each source one after the other
//! in loading order (the default file first), so no source's hook is lost or overwritten.

use indexmap::IndexMap;

use crate::js::JsValue;

/// `HooksSettings`: the matcher groups of each event, in the order the settings wrote them.
pub type HooksSettings = IndexMap<String, Vec<JsValue>>;

/// The hooks of every source, event by event.
#[must_use]
pub fn merge_hooks(sources: &[HooksSettings]) -> HooksSettings {
    let mut merged = HooksSettings::new();
    for source in sources {
        for (event, groups) in source {
            merged
                .entry(event.clone())
                .or_default()
                .extend(groups.iter().cloned());
        }
    }
    merged
}

crate::registry::exports! {
    "loading/merge-hooks" {
        "mergeHooks" => ok |sources: Vec<HooksSettings>| merge_hooks(&sources);
    }
}
