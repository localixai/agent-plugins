//! Port of `src/marketplace/refused-name.ts`, switched.
//!
//! `Claude Code refuses the marketplace name` (errors.md): Claude Code "checks the name every time it
//! reads the marketplace's catalog", so a marketplace registered under an imitating name before the
//! check blocked it stops loading, and so do its plugins. The host calls this on every load beside
//! `ignoredReason` and `untrustedSource`. The advice after the quoted prefix is the kit's wording
//! (spec/judgments.md); the steps for a marketplace declared inline in settings are the docs'
//! (v2.1.292 or later).

use serde::{Deserialize, Serialize};

use crate::format::host_errors::{MARKETPLACE_NAME_LOOKS_OFFICIAL, marketplace_name_refused};
use crate::format::marketplace::marketplace_names::impersonates;

/// The refusal of a marketplace name.
#[derive(Clone, Debug, Serialize)]
pub struct RefusedMarketplaceName {
    /// The marketplace's own error.
    pub marketplace: String,
    /// What `claude plugin list` and the `/plugin` Errors tab show for each of its plugins.
    pub plugin: String,
    /// For a marketplace a settings file declares inline (`"source": "settings"`): the steps that keep it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<String>>,
}

/// What `refusedMarketplaceName` reads of a marketplace source: how it was added.
#[derive(Clone, Debug, Deserialize)]
pub struct SourceKind {
    /// `MarketplaceSourceKind`.
    pub kind: String,
}

fn declared_in_settings(kind: &str) -> bool {
    kind == "settings"
}

fn settings_steps(name: &str) -> Vec<String> {
    vec![
        "In each settings file that lists the marketplace under extraKnownMarketplaces, give the entry's key and its source.name one new plain-ASCII name that doesn't look like an official one. If an administrator adds the marketplace through your organization's managed settings, the administrator does this step".to_owned(),
        format!("Run claude plugin marketplace remove {name}, or remove the old name in the Marketplaces tab in /plugin. This also uninstalls its plugins and deletes their data, options, and secrets"),
        "Start Claude Code again, in the project's folder if the file is a project's settings file. Then install the plugins from the renamed marketplace".to_owned(),
    ]
}

/// The refusal of a registered marketplace whose name imitates an official one, or `undefined` when its name loads.
#[must_use]
pub fn refused_marketplace_name(name: &str, source: &SourceKind) -> Option<RefusedMarketplaceName> {
    if !impersonates(name) {
        return None;
    }
    let plugin = format!(
        "{}: it looks like one of Anthropic's own. Remove it with claude plugin marketplace remove {name}; this also uninstalls its plugins and deletes their data, options and secrets.",
        marketplace_name_refused(name)
    );
    Some(RefusedMarketplaceName {
        marketplace: MARKETPLACE_NAME_LOOKS_OFFICIAL.to_owned(),
        plugin,
        steps: declared_in_settings(&source.kind).then(|| settings_steps(name)),
    })
}

crate::registry::exports! {
    "marketplace/refused-name" {
        "refusedMarketplaceName" => ok |name: String, source: SourceKind|
            refused_marketplace_name(&name, &source);
    }
}
