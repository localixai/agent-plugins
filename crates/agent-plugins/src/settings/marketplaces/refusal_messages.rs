//! Port of `src/settings/marketplaces/refusal-messages.ts`, switched.
//!
//! The policy refusals a user reads (plugins/troubleshooting.md, "Policy refusals"): the line the
//! command prints, one function per entry. The host prints them; the kit only words them.
//!
//! Judgment (spec/judgments.md): the docs give each message's first line and the facts the rest
//! carries, not every word, so the second sentences below follow those facts.

use serde::Deserialize;

/// `Plugins from ~/.claude/skills/ are blocked ...`: `claude plugin init` or `enable` on a skills-directory plugin.
#[must_use]
pub const fn skills_directory_blocked_message() -> &'static str {
    "Plugins from ~/.claude/skills/ are blocked by your organization's managed settings (strictKnownMarketplaces or blockedMarketplaces). \
Ask your administrator to add {\"source\":\"skills-dir\"} to strictKnownMarketplaces or remove it from blockedMarketplaces."
}

/// `Command-sourced plugins are disabled ...`: an install or update of a plugin with a `command` source.
#[must_use]
pub const fn command_source_disabled_message() -> &'static str {
    "Command-sourced plugins are disabled by your organization's managed settings. \
The plugin was not installed or updated and its command was not run."
}

/// What a plugin's own entry is refused for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginAction {
    /// It cannot be installed.
    Install,
    /// It cannot be enabled.
    Enable,
}

/// Why a plugin is refused: its own entry, its marketplace, or a dependency.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PluginBlock {
    /// Its own entry.
    Plugin {
        /// What it cannot do.
        action: PluginAction,
    },
    /// Its marketplace.
    Marketplace {
        /// The marketplace's name.
        marketplace: String,
    },
    /// A dependency.
    Dependency {
        /// The dependency's name.
        dependency: String,
    },
}

const fn ending(action: PluginAction) -> &'static str {
    match action {
        PluginAction::Install => "installed",
        PluginAction::Enable => "enabled",
    }
}

/// What follows the name, per kind of block.
fn describe(block: &PluginBlock) -> String {
    match block {
        PluginBlock::Marketplace { marketplace } => format!(
            "is from marketplace \"{marketplace}\", which is blocked by your organization's policy"
        ),
        PluginBlock::Dependency { dependency } => {
            format!("depends on \"{dependency}\", which is blocked by your organization's policy")
        }
        PluginBlock::Plugin { action } => format!(
            "is blocked by your organization's policy and cannot be {}",
            ending(*action)
        ),
    }
}

/// `Plugin "<name>" is blocked by your organization's policy ...` in its three forms.
#[must_use]
pub fn plugin_blocked_message(name: &str, block: &PluginBlock) -> String {
    format!("Plugin \"{name}\" {}", describe(block))
}

crate::registry::exports! {
    "settings/marketplaces/refusal-messages" {
        "skillsDirectoryBlockedMessage" => ok || skills_directory_blocked_message().to_owned();
        "commandSourceDisabledMessage" => ok || command_source_disabled_message().to_owned();
        "pluginBlockedMessage" => ok |name: String, block: PluginBlock| plugin_blocked_message(&name, &block);
    }
}
