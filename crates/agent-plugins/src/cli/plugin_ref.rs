//! Ported from `src/cli/plugin-ref.ts`; only the core uses it now, so it has no wrapper.
//!
//! The `<plugin>` argument of the plugin commands (plugins/cli-reference.md, "claude plugin
//! commands": "a `<plugin>` argument is a plugin `name` or `name@marketplace`. When two marketplaces
//! offer the same name, use the qualified form"; "plugin update", "Update by bare name": a bare name
//! "matches against your installed plugins"; when the same name is installed from different
//! marketplaces "the command refuses ... and lists the qualified `plugin-name@marketplace-name`
//! commands to run instead"). Pure over the ids the host lists.

use crate::format::marketplace::ids::{parse_plugin_ref, plugin_name_of};

/// The name part of an id (`formatter` of `formatter@my-marketplace`).
#[must_use]
pub fn name_of(id: &str) -> String {
    plugin_name_of(id)
}

/// The ids of `ids` the argument names: a qualified reference matches one id exactly, a bare name
/// every id with that name. No hit is "not installed"; two or more are ambiguous.
#[must_use]
pub fn matching_ids(text: &str, ids: &[String]) -> Vec<String> {
    let Some(reference) = parse_plugin_ref(text) else {
        return Vec::new();
    };
    let mut unique: Vec<&String> = Vec::new();
    for id in ids {
        if !unique.contains(&id) {
            unique.push(id);
        }
    }
    unique
        .into_iter()
        .filter(|id| {
            if reference.marketplace.is_none() {
                name_of(id) == reference.name
            } else {
                id.as_str() == text
            }
        })
        .cloned()
        .collect()
}

/// The refusal of an ambiguous bare name: the qualified commands to run instead.
#[must_use]
pub fn qualified_commands(verb: &str, ids: &[String]) -> Vec<String> {
    ids.iter()
        .map(|id| format!("claude plugin {verb} {id}"))
        .collect()
}
