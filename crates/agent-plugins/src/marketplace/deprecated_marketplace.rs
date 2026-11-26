//! Port of `src/marketplace/deprecated-marketplace.ts`, switched.
//!
//! The deprecated `claude-code-plugins` marketplace (plugins/anthropic-marketplaces.md, "Deprecated
//! `claude-code-plugins` marketplace"): what to run for an install that used to come from it and for the
//! two plugins that changed. Pure; the docs' commands.

use serde::Serialize;

use super::install_scopes::install_command;

const DEPRECATED_MARKETPLACE: &str = "claude-code-plugins";
const OFFICIAL_MARKETPLACE: &str = "claude-plugins-official";

/// Where a plugin of the deprecated marketplace is installed from now: "run `claude plugin install <plugin>@claude-plugins-official`".
#[must_use]
pub fn official_install_command(plugin: &str) -> String {
    install_command(&format!("{plugin}@{OFFICIAL_MARKETPLACE}"))
}

/// The tutorial step to skip.
pub const SKIPPED_TUTORIAL_STEP: &str = "/plugin marketplace add anthropics/claude-code";

/// What changed for a plugin of the deprecated marketplace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginChange {
    /// It has another name.
    Renamed,
    /// It is gone.
    Retired,
}

/// What to do about a plugin of the deprecated marketplace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeprecatedPluginAdvice {
    /// What changed.
    pub change: PluginChange,
    /// The command to run in a shell.
    pub command: String,
    /// For a rename, the prefix the commands have now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_prefix: Option<String>,
}

/// Advice for the two plugins that need it; `None` for every other: "you can leave your plugins and the marketplace as they are".
#[must_use]
pub fn deprecated_plugin_advice(plugin: &str) -> Option<DeprecatedPluginAdvice> {
    match plugin {
        "ralph-wiggum" => Some(DeprecatedPluginAdvice {
            change: PluginChange::Renamed,
            command: official_install_command("ralph-loop"),
            command_prefix: Some("/ralph-loop:".to_owned()),
        }),
        "claude-opus-4-5-migration" => Some(DeprecatedPluginAdvice {
            change: PluginChange::Retired,
            command: format!(
                "claude plugin uninstall claude-opus-4-5-migration@{DEPRECATED_MARKETPLACE}"
            ),
            command_prefix: None,
        }),
        _ => None,
    }
}

crate::registry::exports! {
    "marketplace/deprecated-marketplace" {
        "officialInstallCommand" => ok |plugin: String| official_install_command(&plugin);
        "SKIPPED_TUTORIAL_STEP" => ok || SKIPPED_TUTORIAL_STEP;
        "deprecatedPluginAdvice" => ok |plugin: String| deprecated_plugin_advice(&plugin);
    }
}
