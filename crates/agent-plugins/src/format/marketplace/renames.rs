//! Port of `src/format/marketplace/renames.ts`, switched.
//!
//! The `renames` map of `marketplace.json` (plugins/host-marketplace.md, "Migrate users with a
//! renames map"; plugins/marketplace-reference.md, "Validation messages"): each former plugin name
//! maps to its current name, or to `null` for a plugin that is gone. "Claude Code follows the chain
//! from the oldest name", so a name may map to a name that is itself a key, and the chain must end
//! at `null` or at a name in `plugins` (`claude plugin validate` rejects one that cycles or ends
//! anywhere else).

use indexmap::{IndexMap, IndexSet};
use serde::Serialize;

use crate::format::issues::{PluginIssue, error};
use crate::js::key_order;

/// A former plugin name to its current name, or `None` (`null`) for a plugin that is gone.
pub type Renames = IndexMap<String, Option<String>>;

/// Where a chain from `start` ends. `plugins` is checked before the keys: a name that is still a
/// plugin is where users end up (judgment, spec/judgments.md). Keys are read as own properties only,
/// so `constructor` or `__proto__` in a map or a target is a plain name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "end", rename_all = "lowercase")]
pub enum Chain {
    /// The chain ends at a plugin.
    Plugin {
        /// The plugin's name.
        name: String,
    },
    /// The chain ends at `null`: the plugin is gone.
    Removed,
    /// The chain ends at a name that is neither a plugin nor a key.
    Dangling {
        /// That name.
        name: String,
    },
    /// The chain comes back to a name it has passed.
    Cycle {
        /// The names passed, the repeated one last.
        path: Vec<String>,
    },
}

fn is_key(renames: &Renames, name: &str) -> bool {
    renames.contains_key(name)
}

/// Follows `start` through `renames` to where it ends.
#[must_use]
pub fn follow_renames(renames: &Renames, start: &str, plugins: &IndexSet<String>) -> Chain {
    let mut path = vec![start.to_owned()];
    let mut current = start.to_owned();
    loop {
        let Some(Some(target)) = renames.get(&current) else {
            return Chain::Removed;
        };
        if plugins.contains(target) {
            return Chain::Plugin {
                name: target.clone(),
            };
        }
        if !is_key(renames, target) {
            return Chain::Dangling {
                name: target.clone(),
            };
        }
        if path.contains(target) {
            path.push(target.clone());
            return Chain::Cycle { path };
        }
        path.push(target.clone());
        current.clone_from(target);
    }
}

const UNRESOLVED_TARGET: &str = "target must be a name in plugins[], a key in renames, or null";

/// The reason in the parentheses: `cycle`, or `target-missing` for a chain that ends at no plugin
/// and no key (spec J15.11); `None` for a chain that resolves.
const fn reason(chain: &Chain) -> Option<&'static str> {
    match chain {
        Chain::Dangling { .. } => Some("target-missing"),
        Chain::Cycle { .. } => Some("cycle"),
        Chain::Plugin { .. } | Chain::Removed => None,
    }
}

/// What `claude plugin validate` reports about `renames`, on `renames.<old>`: a chain that cycles or
/// ends at a name that is neither a plugin nor a key. A target is held to the id grammar only when it
/// names neither an entry nor a key, and such a target already ends the chain as `target-missing`,
/// so `"bad name"` draws that reason and a target that is itself a key (`"bad key"`) is followed
/// (spec J15.11, oracle 2.1.325): the docs' PluginIdSchema row is never the one printed.
#[must_use]
pub fn validate_renames(renames: &Renames, plugin_names: &[String]) -> Vec<PluginIssue> {
    let plugins: IndexSet<String> = plugin_names.iter().cloned().collect();
    let mut found = Vec::new();
    // `Object.keys` lists integer-like names first (`js::key_order`).
    for old in key_order(renames.keys()) {
        let chain = follow_renames(renames, old, &plugins);
        if let Some(why) = reason(&chain) {
            found.push(error(
                &format!("renames.{old}"),
                &format!("chain does not resolve ({why}) — {UNRESOLVED_TARGET}"),
            ));
        }
    }
    found
}

crate::registry::exports! {
    "format/marketplace/renames" {
        "validateRenames" => ok |renames: Renames, plugin_names: Vec<String>| validate_renames(&renames, &plugin_names);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'import { validateRenames } from "./src/format/marketplace/renames.ts";
    //   console.log(JSON.stringify(validateRenames(JSON.parse(`{"b":"nowhere","10":"gone",
    //   "2":"gone2","a":null}`), []).map((issue) => issue.path)))'
    // prints ["renames.2","renames.10","renames.b"]
    #[test]
    fn errors_come_in_the_order_object_keys_give() {
        let renames: Renames = [
            ("b", Some("nowhere")),
            ("10", Some("gone")),
            ("2", Some("gone2")),
            ("a", None),
        ]
        .into_iter()
        .map(|(old, target)| (old.to_owned(), target.map(str::to_owned)))
        .collect();
        let paths: Vec<String> = validate_renames(&renames, &[])
            .into_iter()
            .map(|issue| issue.path)
            .collect();
        assert_eq!(paths, ["renames.2", "renames.10", "renames.b"]);
    }
}
