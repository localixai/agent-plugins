//! Port of `src/settings/channels/verdict.ts`, switched.
//!
//! Whether a channel plugin a user passed for the session may register: the gate first, then the
//! rule of the flag the entry came from.

use serde::{Deserialize, Serialize};

use super::allowlist::{AllowlistSource, ChannelAllowlist, ChannelPluginRef, is_listed};
use super::channel_gate::ChannelGate;

/// Why a channel plugin may or may not register.
pub const CHANNEL_REASONS: [ChannelReason; 7] = [
    ChannelReason::OrganizationAllowlist,
    ChannelReason::DefaultAllowlist,
    ChannelReason::DevelopmentFlag,
    ChannelReason::ChannelsNotEnabled,
    ChannelReason::NotOnOrganizationList,
    ChannelReason::NotOnDefaultList,
    ChannelReason::DevelopmentFlagNeedsInteractive,
];

/// One of `CHANNEL_REASONS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelReason {
    /// On the organization's list.
    OrganizationAllowlist,
    /// On the default list.
    DefaultAllowlist,
    /// Loaded with the development flag.
    DevelopmentFlag,
    /// The gate is closed.
    ChannelsNotEnabled,
    /// Not on the organization's list.
    NotOnOrganizationList,
    /// Not on the default list.
    NotOnDefaultList,
    /// The development flag outside an interactive session.
    DevelopmentFlagNeedsInteractive,
}

/// Which reasons let the channel register.
const fn admitted(reason: ChannelReason) -> bool {
    match reason {
        ChannelReason::OrganizationAllowlist
        | ChannelReason::DefaultAllowlist
        | ChannelReason::DevelopmentFlag => true,
        ChannelReason::ChannelsNotEnabled
        | ChannelReason::NotOnOrganizationList
        | ChannelReason::NotOnDefaultList
        | ChannelReason::DevelopmentFlagNeedsInteractive => false,
    }
}

/// Whether the channel may register, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ChannelVerdict {
    /// Whether it registers.
    pub allowed: bool,
    /// Why.
    pub reason: ChannelReason,
}

/// The two flags that carry a channel into a session.
pub const CHANNEL_ORIGINS: [&str; 2] = ["channels", "development"];

/// One entry a user passed for the session, with the flag it came from. "The bypass is
/// per-entry": the development flag skips the list for its own entries only, and "doesn't extend
/// the bypass to the `--channels` entries" (channels-reference.md, "Test during the research
/// preview"). Either way the user opted in: "no channel runs until a user opts it in for the
/// session" (channels.md, "Enterprise controls").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "origin", rename_all = "camelCase")]
pub enum ChannelEntry {
    /// `--channels plugin:<name>@<marketplace>`: faces the allowlist.
    Channels {
        /// The plugin.
        plugin: ChannelPluginRef,
    },
    /// `--dangerously-load-development-channels`, `plugin:<name>@<marketplace>` or a bare
    /// `server:<name>`: skips the allowlist, so the verdict reads no plugin; it is set for the
    /// first form.
    Development {
        /// Whether the session is interactive.
        interactive: bool,
        /// The plugin, for the first form.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        plugin: Option<ChannelPluginRef>,
    },
}

const fn listed_reason(source: AllowlistSource) -> [ChannelReason; 2] {
    match source {
        AllowlistSource::Organization => [
            ChannelReason::OrganizationAllowlist,
            ChannelReason::NotOnOrganizationList,
        ],
        AllowlistSource::Default => [
            ChannelReason::DefaultAllowlist,
            ChannelReason::NotOnDefaultList,
        ],
    }
}

fn reason_for(entry: &ChannelEntry, allowlist: &ChannelAllowlist) -> ChannelReason {
    match entry {
        ChannelEntry::Channels { plugin } => {
            let [listed, missing] = listed_reason(allowlist.source);
            if is_listed(&allowlist.entries, plugin) {
                listed
            } else {
                missing
            }
        }
        // Run in an interactive session, where Claude Code can show the confirmation prompt;
        // elsewhere it "ignores the flag and the channel doesn't register".
        ChannelEntry::Development { interactive, .. } => {
            if *interactive {
                ChannelReason::DevelopmentFlag
            } else {
                ChannelReason::DevelopmentFlagNeedsInteractive
            }
        }
    }
}

const fn verdict(reason: ChannelReason) -> ChannelVerdict {
    ChannelVerdict {
        allowed: admitted(reason),
        reason,
    }
}

/// The gate first (it blocks the development flag too), then the entry's own flag rule.
#[must_use]
pub fn channel_verdict(
    gate: &ChannelGate,
    allowlist: &ChannelAllowlist,
    entry: &ChannelEntry,
) -> ChannelVerdict {
    verdict(if gate.open {
        reason_for(entry, allowlist)
    } else {
        ChannelReason::ChannelsNotEnabled
    })
}

crate::registry::exports! {
    "settings/channels/verdict" {
        "CHANNEL_REASONS" => ok || CHANNEL_REASONS;
        "CHANNEL_ORIGINS" => ok || CHANNEL_ORIGINS;
        "channelVerdict" => ok |gate: ChannelGate, allowlist: ChannelAllowlist, entry: ChannelEntry|
            channel_verdict(&gate, &allowlist, &entry);
    }
}
