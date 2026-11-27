//! Port of `src/settings/channels/policy.ts`, switched.

use crate::registry::{Handle, Port};
use crate::settings::channels::allowlist::{
    ChannelAllowlist, ChannelAllowlistContext, effective_channel_allowlist,
};
use crate::settings::channels::channel_gate::{ChannelGate, ChannelsContext, channel_gate};
use crate::settings::channels::verdict::{ChannelEntry, ChannelVerdict, channel_verdict};
use crate::settings::view::PolicyView;

/// What the host tells the channel keys besides the layers.
pub type ChannelPolicyContext = ChannelAllowlistContext;

/// The channel keys of the settings as one object over a view (docs/API.md, "Settings").
#[derive(Debug)]
pub struct ChannelSettingsPolicy {
    /// `channelsEnabled`: whether channels may deliver messages on this account.
    pub gate: ChannelGate,
    /// `allowedChannelPlugins` over the host's default list.
    pub allowlist: ChannelAllowlist,
}

impl ChannelSettingsPolicy {
    /// Whether an entry the user passed for the session may register as a channel.
    #[must_use]
    pub fn allowed(&self, entry: &ChannelEntry) -> ChannelVerdict {
        channel_verdict(&self.gate, &self.allowlist, entry)
    }
}

/// The policy over `view`: the gate and the allowlist are read once.
pub async fn create_channel_settings_policy(
    view: &dyn PolicyView,
    context: &ChannelPolicyContext,
) -> ChannelSettingsPolicy {
    let gate_context = ChannelsContext {
        account: context.account,
        has_managed_settings: context.has_managed_settings,
    };
    let gate = channel_gate(view, &gate_context).await;
    let allowlist = effective_channel_allowlist(view, context).await;
    ChannelSettingsPolicy { gate, allowlist }
}

crate::registry::methods! {
    ChannelSettingsPolicy {
        "gate" => ok |this| this.gate.clone();
        "allowlist" => ok |this| this.allowlist.clone();
        "allowed" => ok |this, entry: ChannelEntry| this.allowed(&entry);
    }
}

crate::registry::exports! {
    "settings/channels/policy" {
        "createChannelSettingsPolicy" => async ok |view: Port, context: ChannelPolicyContext|
            Handle::new(create_channel_settings_policy(&view, &context).await);
    }
}
