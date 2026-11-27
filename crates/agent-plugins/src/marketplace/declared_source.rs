//! Port of `src/marketplace/declared-source.ts`, switched.
//!
//! Adding a marketplace whose `name` already has an `extraKnownMarketplaces` entry in user or managed
//! settings (plugins/troubleshooting.md, "Cannot add marketplace "`<name>`": its source doesn't match its
//! extraKnownMarketplaces entry in user or managed settings"): the add is refused and registers nothing
//! unless the sources match. Pure; the host reads the entry.

use serde::Deserialize;

use super::known_marketplaces::same_marketplace_source;
use crate::format::marketplace::types::MarketplaceSource;

/// The text the docs quote since v2.1.287.
#[must_use]
pub fn declared_source_mismatch(name: &str) -> String {
    format!(
        "Cannot add marketplace \"{name}\": its source doesn't match its extraKnownMarketplaces entry in user or managed settings"
    )
}

/// What the host knows about the add.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DeclaredSourceFacts {
    /// The marketplace's name.
    pub name: String,
    /// The source of the add, as `parseMarketplaceInput` built it.
    pub given: MarketplaceSource,
    /// The source the user or managed `extraKnownMarketplaces` entry lists for that name; absent when none does.
    #[serde(default)]
    pub declared: Option<MarketplaceSource>,
}

/// "Two sources match when they have the same type and the same value in every field. An entry that sets a
/// `ref` you didn't pass counts as different. A `github` entry also counts as different when you gave the
/// repository as an `https://github.com/` URL, because Claude Code records that URL as a `git` source."
/// `undefined` when there is nothing to refuse.
#[must_use]
pub fn declared_source_refusal(facts: &DeclaredSourceFacts) -> Option<String> {
    match &facts.declared {
        None => None,
        Some(declared) if same_marketplace_source(&facts.given, declared) => None,
        Some(_) => Some(declared_source_mismatch(&facts.name)),
    }
}

/// The fix the docs give first: "Use the declared source", typed as the entry writes it.
#[must_use]
pub fn declared_source_fix(declared: &MarketplaceSource) -> Option<String> {
    let MarketplaceSource::Github {
        repo, reference, ..
    } = declared
    else {
        return None;
    };
    let reference = reference
        .as_ref()
        .map_or_else(String::new, |reference| format!("#{reference}"));
    Some(format!("/plugin marketplace add {repo}{reference}"))
}

crate::registry::exports! {
    "marketplace/declared-source" {
        "declaredSourceMismatch" => ok |name: String| declared_source_mismatch(&name);
        "declaredSourceRefusal" => ok |facts: DeclaredSourceFacts| declared_source_refusal(&facts);
        "declaredSourceFix" => ok |declared: MarketplaceSource| declared_source_fix(&declared);
    }
}
