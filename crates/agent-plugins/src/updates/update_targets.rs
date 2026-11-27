//! Port of `src/updates/update-targets.ts`, switched.
//!
//! "Update plugins now" (plugins/install.md): what each way of updating touches. Pure: the host
//! refreshes listings and updates plugins; the kit says which, and which it leaves to the user.
//! "There's no command that updates every plugin at once", so no request here asks for that.

use serde::{Deserialize, Serialize};

/// A plugin the user installed, with what decides whether a bulk update may touch it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledForUpdate {
    /// `name@marketplace`.
    pub id: String,
    /// Its marketplace.
    pub marketplace: String,
    /// Its marketplace entry has a `command` source.
    #[serde(default)]
    pub command_source: Option<bool>,
    /// Its marketplace entry sets a `headersHelper` command.
    #[serde(default)]
    pub headers_helper: Option<String>,
}

/// What the user asked to update.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind")]
pub enum UpdateRequest {
    /// `Update now` on the Installed tab, or `claude plugin update <plugin>@<marketplace>`.
    #[serde(rename = "plugin")]
    Plugin {
        /// The plugin.
        id: String,
    },
    /// `Update marketplace` on the Marketplaces tab.
    #[serde(rename = "marketplace-tab")]
    MarketplaceTab {
        /// The marketplace.
        marketplace: String,
    },
    /// `claude plugin marketplace update [name]`, and the in-session command that opens the tab: the listing only.
    #[serde(rename = "marketplace-listing")]
    MarketplaceListing {
        /// The marketplace, or every one.
        #[serde(default)]
        marketplace: Option<String>,
    },
}

/// Why a plugin is left to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LeftReason {
    /// `command-source`.
    #[serde(rename = "command-source")]
    CommandSource,
    /// `headers-helper`.
    #[serde(rename = "headers-helper")]
    HeadersHelper,
}

/// A plugin a bulk update left for the user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LeftToUser {
    /// The plugin.
    pub id: String,
    /// Why.
    pub reason: LeftReason,
    /// "you update it from its view on the Installed tab or with `claude plugin update <plugin>@<marketplace>`".
    pub instruction: String,
}

/// What an update does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlan {
    /// Marketplaces whose listing is refreshed.
    pub refresh: Vec<String>,
    /// Plugin ids updated.
    pub update: Vec<String>,
    /// Plugins a bulk update left for the user to update themselves.
    pub left_to_user: Vec<LeftToUser>,
}

/// A bulk update does not touch a `command` source or a `headersHelper` entry: both need the user's acceptance.
fn left_reason(plugin: &InstalledForUpdate) -> Option<LeftReason> {
    if plugin.headers_helper.is_some() {
        return Some(LeftReason::HeadersHelper);
    }
    (plugin.command_source == Some(true)).then_some(LeftReason::CommandSource)
}

fn is_registered(name: &str, registered: &[String]) -> bool {
    registered.iter().any(|candidate| candidate == name)
}

/// The listing of `marketplace`, then the plugins installed from it, less the ones left to the user.
fn marketplace_tab(
    marketplace: &str,
    installed: &[InstalledForUpdate],
    registered: &[String],
) -> UpdatePlan {
    if !is_registered(marketplace, registered) {
        return UpdatePlan {
            refresh: Vec::new(),
            update: Vec::new(),
            left_to_user: Vec::new(),
        };
    }
    let mine: Vec<&InstalledForUpdate> = installed
        .iter()
        .filter(|plugin| plugin.marketplace == marketplace)
        .collect();
    let left = mine
        .iter()
        .filter_map(|plugin| {
            left_reason(plugin).map(|reason| LeftToUser {
                id: plugin.id.clone(),
                reason,
                instruction: format!("claude plugin update {}", plugin.id),
            })
        })
        .collect();
    UpdatePlan {
        refresh: vec![marketplace.to_owned()],
        update: mine
            .iter()
            .filter(|plugin| left_reason(plugin).is_none())
            .map(|plugin| plugin.id.clone())
            .collect(),
        left_to_user: left,
    }
}

/// "It refreshes every marketplace's listing but leaves your installed plugins at their current versions."
fn listing_only(marketplace: Option<&str>, registered: &[String]) -> UpdatePlan {
    let refresh = match marketplace {
        None => registered.to_vec(),
        Some(marketplace) => registered
            .iter()
            .filter(|name| name.as_str() == marketplace)
            .cloned()
            .collect(),
    };
    UpdatePlan {
        refresh,
        update: Vec::new(),
        left_to_user: Vec::new(),
    }
}

/// What `request` does. Judgments (spec/judgments.md): a single-plugin update names no marketplace
/// refresh; a plugin that is not installed has nothing to update; a marketplace that is not
/// registered has nothing to refresh; a plugin with both a
/// `headersHelper` and a `command` source is reported as a `headersHelper` one.
#[must_use]
pub fn plan_update(
    request: &UpdateRequest,
    installed: &[InstalledForUpdate],
    registered: &[String],
) -> UpdatePlan {
    match request {
        UpdateRequest::Plugin { id } => UpdatePlan {
            refresh: Vec::new(),
            update: if installed.iter().any(|plugin| plugin.id == *id) {
                vec![id.clone()]
            } else {
                Vec::new()
            },
            left_to_user: Vec::new(),
        },
        UpdateRequest::MarketplaceTab { marketplace } => {
            marketplace_tab(marketplace, installed, registered)
        }
        UpdateRequest::MarketplaceListing { marketplace } => {
            listing_only(marketplace.as_deref(), registered)
        }
    }
}

crate::registry::exports! {
    "updates/update-targets" {
        "planUpdate" => ok |request: UpdateRequest, installed: Vec<InstalledForUpdate>, registered: Vec<String>| plan_update(&request, &installed, &registered);
    }
}
