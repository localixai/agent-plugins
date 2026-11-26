//! Port of `src/updates/auto-update-skips.ts`, switched.
//!
//! The plugins the auto-update pass leaves alone (plugins/loading.md, "Which marketplaces and
//! plugins auto-update": "Auto-update also skips a plugin whose marketplace entry declares a
//! `headersHelper`"; "When a command source re-runs": a command source "doesn't wait for the
//! auto-update pass"; plugins/host-marketplace.md, "Installs and updates that refuse a command instead
//! of asking": a background auto-update "lists the plugin in the `/plugin` Errors tab").

use serde::{Deserialize, Serialize};

/// A plugin the pass looks at.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateCandidate {
    /// The marketplace entry declares a `headersHelper` command.
    #[serde(default)]
    pub headers_helper: Option<String>,
    /// The plugin has a `command` source; `accepted` says the user accepted the command it now declares.
    #[serde(default)]
    pub command_source: Option<CommandSource>,
}

/// A plugin's `command` source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct CommandSource {
    /// The user accepted the command it now declares.
    pub accepted: bool,
}

/// Why a plugin is skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SkippedBy {
    /// `headers-helper`.
    #[serde(rename = "headers-helper")]
    HeadersHelper,
    /// `command-needs-acceptance`.
    #[serde(rename = "command-needs-acceptance")]
    CommandNeedsAcceptance,
    /// `command-source`.
    #[serde(rename = "command-source")]
    CommandSource,
}

/// Whether the pass skips the plugin, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum AutoUpdateSkip {
    /// `{ skipped: false }`.
    Kept {
        /// Always false.
        skipped: bool,
    },
    /// `{ skipped: true, reason, listedInErrorsTab }`.
    Skipped {
        /// Always true.
        skipped: bool,
        /// Why.
        reason: SkippedBy,
        /// The user is told in the Errors tab to install or update it by hand.
        #[serde(rename = "listedInErrorsTab")]
        listed_in_errors_tab: bool,
    },
}

const fn skip(reason: SkippedBy, listed_in_errors_tab: bool) -> AutoUpdateSkip {
    AutoUpdateSkip::Skipped {
        skipped: true,
        reason,
        listed_in_errors_tab,
    }
}

/// Whether the pass skips the plugin. Judgment (spec/judgments.md): a command source the user
/// accepted is skipped because its command re-runs on its own schedule; one whose command changed or
/// was never accepted is listed in the Errors tab, like a `headersHelper` plugin.
#[must_use]
pub const fn plugin_skipped_by_auto_update(candidate: &AutoUpdateCandidate) -> AutoUpdateSkip {
    if candidate.headers_helper.is_some() {
        return skip(SkippedBy::HeadersHelper, true);
    }
    match candidate.command_source {
        None => AutoUpdateSkip::Kept { skipped: false },
        Some(CommandSource { accepted: true }) => skip(SkippedBy::CommandSource, false),
        Some(CommandSource { accepted: false }) => skip(SkippedBy::CommandNeedsAcceptance, true),
    }
}

crate::registry::exports! {
    "updates/auto-update-skips" {
        "pluginSkippedByAutoUpdate" => ok |candidate: AutoUpdateCandidate| plugin_skipped_by_auto_update(&candidate);
    }
}
