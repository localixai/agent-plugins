//! Port of `src/settings/channels/channel-gate.ts` at sha256:c854da77d58d.
//!
//! The account kinds the `channelsEnabled` default depends on (settings-reference.md,
//! `channelsEnabled`; channels.md, "Enterprise controls"). The host tells which one the session is
//! signed in with. Takes the host's `PolicyView`: stays TypeScript in the package (the module
//! comment of `crate::settings`), so only its data names are exported.

use serde::{Deserialize, Serialize};

use crate::settings::layers::PolicyScope;
use crate::settings::undefined_if_none;
use crate::settings::view::PolicyView;

/// The account kinds the `channelsEnabled` default depends on.
pub const CHANNEL_ACCOUNTS: [ChannelAccount; 3] = [
    ChannelAccount::ClaudeaiTeamEnterprise,
    ChannelAccount::ClaudeaiPersonal,
    ChannelAccount::Console,
];

/// One of `CHANNEL_ACCOUNTS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelAccount {
    /// A Team or Enterprise claude.ai account.
    ClaudeaiTeamEnterprise,
    /// A personal claude.ai account (Pro, Max, Ultra).
    ClaudeaiPersonal,
    /// A Console account.
    Console,
}

/// What the host tells the channel gate besides the layers: plain data, the kit reads no
/// environment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelsContext {
    /// The account the session is signed in with.
    pub account: ChannelAccount,
    /// An administrator deploys managed settings on this machine or account. A managed layer among
    /// the view's layers counts as managed settings too, so a host that cannot tell may pass
    /// `false`.
    pub has_managed_settings: bool,
}

/// What opened the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelGateBy {
    /// An account without an organization.
    NoOrganization,
    /// A Console account's default.
    ConsoleDefault,
    /// `channelsEnabled` is `true`.
    ChannelsEnabled,
}

/// The warning a closed gate asks the host to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupWarning {
    /// "A startup warning tells the user to have an admin enable the setting" (channels.md).
    AdminMustEnable,
}

/// Whether channels may deliver messages, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelGate {
    /// Whether the gate is open.
    pub open: bool,
    /// Why the gate is open; `undefined` while it is closed.
    #[serde(serialize_with = "undefined_if_none")]
    pub by: Option<ChannelGateBy>,
    /// "A startup warning tells the user to have an admin enable the setting" (channels.md,
    /// "Enable channels for your organization"). The wording belongs to the host; this names the
    /// warning.
    #[serde(serialize_with = "undefined_if_none")]
    pub startup_warning: Option<StartupWarning>,
    /// Always `true`: a closed gate stops channel messages only. "The MCP server still connects and
    /// its tools work, but channel messages won't arrive" (channels.md, "Enable channels for your
    /// organization"). The kit starts no server; this tells the host not to disconnect one.
    pub server_connects: bool,
}

/// What a gate rule needs: whether `channelsEnabled` is `true` and whether managed settings exist.
struct GateInput {
    enabled: bool,
    managed: bool,
}

fn gate_rule(account: ChannelAccount, input: &GateInput) -> Option<ChannelGateBy> {
    match account {
        // "Pro, Max, and Ultra users without an organization skip these checks entirely."
        ChannelAccount::ClaudeaiPersonal => Some(ChannelGateBy::NoOrganization),
        // "channels are blocked until an Owner enables them."
        ChannelAccount::ClaudeaiTeamEnterprise => {
            input.enabled.then_some(ChannelGateBy::ChannelsEnabled)
        }
        // "permitted by default ... If your organization deploys managed settings ... blocked
        // until this key is set."
        ChannelAccount::Console => {
            if input.enabled {
                return Some(ChannelGateBy::ChannelsEnabled);
            }
            (!input.managed).then_some(ChannelGateBy::ConsoleDefault)
        }
    }
}

/// The master switch: whether channels may deliver messages on this account. `channelsEnabled:
/// false` is "the same as unset" and the key counts from managed settings only (its Scope line,
/// enforced by the view), so a user cannot lift the block. A closed gate blocks every channel, the
/// development flag included (channels-reference.md, "Test during the research preview").
#[must_use]
pub fn channel_gate(view: &dyn PolicyView, context: &ChannelsContext) -> ChannelGate {
    let managed = context.has_managed_settings
        || view
            .layers()
            .iter()
            .any(|layer| layer.scope == PolicyScope::Managed);
    let by = gate_rule(
        context.account,
        &GateInput {
            enabled: view
                .get("channelsEnabled")
                .and_then(|value| value.as_bool())
                == Some(true),
            managed,
        },
    );
    ChannelGate {
        open: by.is_some(),
        by,
        startup_warning: if by.is_none() {
            Some(StartupWarning::AdminMustEnable)
        } else {
            None
        },
        server_connects: true,
    }
}

crate::registry::exports! {
    "settings/channels/channel-gate" {
        "CHANNEL_ACCOUNTS" => ok || CHANNEL_ACCOUNTS;
    }
}
