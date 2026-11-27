//! Port of `src/settings/channels/allowlist.ts`, switched.
//!
//! The channel allowlist in force.

use serde::{Deserialize, Serialize};

use super::channel_gate::ChannelAccount;
use crate::registry::Port;
use crate::settings::view::PolicyView;

/// A channel plugin as an allowlist names it (the official `ChannelPluginRef`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelPluginRef {
    /// The plugin's name.
    pub plugin: String,
    /// The marketplace it comes from.
    pub marketplace: String,
}

/// What the host tells the allowlist besides the layers.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAllowlistContext {
    /// The account the session is signed in with.
    pub account: ChannelAccount,
    /// An administrator deploys managed settings on this machine or account.
    pub has_managed_settings: bool,
    /// The Anthropic-maintained default allowlist. Its content is not in the docs ("the channel
    /// plugins in claude-plugins-official are the default approved set", channels.md), so the host
    /// supplies it.
    pub default_channel_allowlist: Vec<ChannelPluginRef>,
}

/// Whether the organization's list replaced the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AllowlistSource {
    /// `allowedChannelPlugins` replaced the default.
    Organization,
    /// The Anthropic-maintained default.
    Default,
}

/// The allowlist in force.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAllowlist {
    /// `organization` when `allowedChannelPlugins` replaced the default, `default` otherwise.
    pub source: AllowlistSource,
    /// The plugins the list names.
    pub entries: Vec<ChannelPluginRef>,
}

/// Whether an account reads `allowedChannelPlugins`: Pro, Max and Ultra without an organization
/// "skip these checks entirely" (channels.md, "Enterprise controls"), so they stay on the default
/// list.
const fn reads_organization_list(account: ChannelAccount) -> bool {
    match account {
        ChannelAccount::ClaudeaiTeamEnterprise | ChannelAccount::Console => true,
        ChannelAccount::ClaudeaiPersonal => false,
    }
}

/// The allowlist in force. `allowedChannelPlugins` "replaces the Anthropic-maintained list when
/// set", an empty list included, and is taken whole from the highest managed source
/// (managed-settings.md, "Restriction allowlists"; the view's merge). Entries are `{ plugin,
/// marketplace }`: the string `plugin@marketplace` was read as the equivalent object when the layer
/// was read. An account without an organization skips the key (channels.md, "Enterprise controls";
/// spec/judgments.md).
pub async fn effective_channel_allowlist(
    view: &dyn PolicyView,
    context: &ChannelAllowlistContext,
) -> ChannelAllowlist {
    let organization = if reads_organization_list(context.account) {
        view.get("allowedChannelPlugins").await
    } else {
        None
    };
    match organization {
        None => ChannelAllowlist {
            source: AllowlistSource::Default,
            entries: context.default_channel_allowlist.clone(),
        },
        Some(entries) => ChannelAllowlist {
            source: AllowlistSource::Organization,
            entries: Vec::<ChannelPluginRef>::deserialize(entries).unwrap_or_default(),
        },
    }
}

/// Whether the list names this plugin from this marketplace: both names exactly, case included.
#[must_use]
pub fn is_listed(list: &[ChannelPluginRef], reference: &ChannelPluginRef) -> bool {
    list.iter()
        .any(|entry| entry.plugin == reference.plugin && entry.marketplace == reference.marketplace)
}

crate::registry::exports! {
    "settings/channels/allowlist" {
        "effectiveChannelAllowlist" => async ok |view: Port, context: ChannelAllowlistContext|
            effective_channel_allowlist(&view, &context).await;
        "isListed" => ok |list: Vec<ChannelPluginRef>, reference: ChannelPluginRef| is_listed(&list, &reference);
    }
}
