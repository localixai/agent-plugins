//! Port of `src/marketplace/seed-registry.ts`, switched.
//!
//! How a seed's marketplaces enter the registry (plugins/org.md, "Seed containers and CI"): "Claude
//! Code registers the marketplace from these directories at startup"; "Claude Code uses the first seed
//! that contains a given marketplace"; "a marketplace declared in the seed overwrites the user's entry
//! of the same name"; the seed is read-only and "forces `autoUpdate` off for seed marketplaces".
//! Pure: the host reads `known_marketplaces.json` of each directory and applies the result.
//!
//! An entry is any object the host keeps (`E extends SeededEntry`): the kit reads only its
//! `autoUpdate`, and everything else it holds goes through as it came, so an entry is a
//! [`JsObject`] here.

use std::collections::HashSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::value::{JsObject, JsValue};
use crate::js::{in_key_order, property_order};

/// One seed directory and the marketplaces its `known_marketplaces.json` declares.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seed {
    /// The seed directory.
    pub directory: String,
    /// The marketplaces it declares.
    pub marketplaces: JsObject,
}

/// A marketplace of the registry and where it came from.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    /// The entry.
    pub entry: JsValue,
    /// The seed directory that supplies it; the entry is read-only while this is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_directory: Option<String>,
}

/// `{ ...entry, autoUpdate: false }`: the members spread in order, `autoUpdate` set in place or last.
fn without_auto_update(entry: &JsValue) -> JsValue {
    let mut members = entry.as_object().cloned().unwrap_or_default();
    members.insert("autoUpdate".to_owned(), JsValue::Bool(false));
    JsValue::Object(members)
}

/// The registry after the seeds: every user entry, replaced where a seed declares the same name, and
/// every seed marketplace the user has no entry for. `seeds` is in the order of the path list.
#[must_use]
pub fn apply_seeds(user: &JsObject, seeds: &[Seed]) -> IndexMap<String, RegistryEntry> {
    let mut registry: IndexMap<String, RegistryEntry> = property_order(user)
        .into_iter()
        .map(|(name, entry)| {
            (
                name.clone(),
                RegistryEntry {
                    entry: entry.clone(),
                    seed_directory: None,
                },
            )
        })
        .collect();
    let mut claimed: HashSet<&str> = HashSet::new();
    for seed in seeds {
        for (name, entry) in property_order(&seed.marketplaces) {
            if !claimed.insert(name) {
                continue;
            }
            registry.insert(
                name.clone(),
                RegistryEntry {
                    entry: without_auto_update(entry),
                    seed_directory: Some(seed.directory.clone()),
                },
            );
        }
    }
    in_key_order(registry)
}

crate::registry::exports! {
    "marketplace/seed-registry" {
        "applySeeds" => ok |user: JsObject, seeds: Vec<Seed>| apply_seeds(&user, &seeds);
    }
}
