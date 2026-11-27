//! Port of `src/loading/core/entry-fields.ts`, switched.
//!
//! What a marketplace entry says that a core package does not read (Agent Plugins Extended §18.1.5
//! item 2, J18.15): its component fields and `strict: false` are Claude Code configuration, and a
//! core package has fixed locations and nothing configured beside them (core §5.1, §6.1). Each is
//! reported in the package's errors and the package loads as the core says; the entry's metadata
//! applies as for any entry. Judgment: every component key the entry reader keeps is reported, the
//! chapter's eight and `userConfig`, `channels`, `settings` and `workflows`, none of which a core
//! package reads either.

use indexmap::IndexMap;
use serde::Deserialize;

use crate::format::issues::{PluginIssue, error};
use crate::js::JsValue;

/// The part of a marketplace entry this module reads.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MarketplaceEntry {
    /// The entry's name.
    pub name: String,
    /// The component fields the entry kept, by field.
    pub components: IndexMap<String, JsValue>,
    /// `strict`.
    pub strict: bool,
}

/// One error per component field of the entry, then one for `strict: false`.
#[must_use]
pub fn core_entry_issues(entry: Option<&MarketplaceEntry>) -> Vec<PluginIssue> {
    let Some(entry) = entry else {
        return Vec::new();
    };
    let mut issues: Vec<PluginIssue> = entry
        .components
        .keys()
        .map(|field| {
            error(
                field,
                &format!(
                    "marketplace entry \"{}\": {field} does not apply to a core package, which loads only its own fixed locations",
                    entry.name
                ),
            )
        })
        .collect();
    if !entry.strict {
        issues.push(error(
            "strict",
            &format!(
                "marketplace entry \"{}\": strict: false does not apply to a core package, whose plugin.json is always its manifest",
                entry.name
            ),
        ));
    }
    issues
}

crate::registry::exports! {
    "loading/core/entry-fields" {
        "coreEntryIssues" => ok |entry: Option<MarketplaceEntry>| core_entry_issues(entry.as_ref());
    }
}
