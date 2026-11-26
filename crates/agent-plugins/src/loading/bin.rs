//! Port of `src/loading/bin.ts`, switched.
//!
//! Executables (plugins/manifest-reference.md, "Standard layout"; plugins/components.md,
//! "Executables"). Putting a directory on the Bash tool's `PATH` is the host's effect; what the kit
//! owns is which directory, where it goes among the user's own entries, and which hosts take a plugin
//! that has one.

use serde::{Deserialize, Serialize};

use crate::ports::locations::Locations;
use crate::registry::Port;

/// What these read of a plugin (`Pick<Plugin, 'hasBin'>`).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HasBin {
    /// The plugin has a top-level `bin/` directory.
    pub has_bin: bool,
}

/// The absolute `bin/` directory of a plugin, only when it has one ("while the plugin is enabled").
pub async fn bin_path_entries(
    plugin: HasBin,
    plugin_id: &str,
    locations: &impl Locations,
) -> Vec<String> {
    if !plugin.has_bin {
        return Vec::new();
    }
    let root = locations.plugin_root(plugin_id).await;
    vec![locations.join_path(&root, "bin").await]
}

/// The `PATH` entries with plugin `bin/` directories added: "Plugin `bin/` directories come after the
/// user's own `PATH` entries, so a plugin can't shadow `git`, `ls`, or another system command". A
/// directory the user already lists keeps its own position.
#[must_use]
pub fn path_with_plugin_bins(user_entries: &[String], bin_directories: &[String]) -> Vec<String> {
    let mut entries = user_entries.to_vec();
    let mut added: Vec<&String> = Vec::new();
    for directory in bin_directories {
        if !user_entries.contains(directory) && !added.contains(&directory) {
            added.push(directory);
        }
    }
    entries.extend(added.into_iter().cloned());
    entries
}

/// Where a plugin can be installed: Claude Code and the two hosted surfaces the docs name.
pub const DISTRIBUTION_TARGETS: [DistributionTarget; 3] = [
    DistributionTarget::ClaudeCode,
    DistributionTarget::ClaudeAi,
    DistributionTarget::Cowork,
];

/// One of `DISTRIBUTION_TARGETS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DistributionTarget {
    /// Claude Code.
    ClaudeCode,
    /// claude.ai.
    ClaudeAi,
    /// Cowork.
    Cowork,
}

/// Whether a target installs the plugin, and why not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Distribution {
    /// Whether it installs.
    pub installable: bool,
    /// Why it does not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

const INSTALLABLE: Distribution = Distribution {
    installable: true,
    reason: None,
};

/// `HOSTED_REFUSAL`: the refusal of a hosted surface.
fn hosted_refusal(surface: &str) -> Distribution {
    Distribution {
        installable: false,
        reason: Some(format!(
            "{surface} doesn't install a plugin that has a top-level bin/ directory"
        )),
    }
}

/// One strategy per target: "claude.ai and Cowork don't install a plugin that has this directory,
/// including one you distribute through claude.ai organization settings". The target names are the
/// kit's wording of that sentence (judgment).
fn targets(target: DistributionTarget, plugin: HasBin) -> Distribution {
    match target {
        DistributionTarget::ClaudeCode => INSTALLABLE,
        DistributionTarget::ClaudeAi if plugin.has_bin => hosted_refusal("claude.ai"),
        DistributionTarget::Cowork if plugin.has_bin => hosted_refusal("Cowork"),
        DistributionTarget::ClaudeAi | DistributionTarget::Cowork => INSTALLABLE,
    }
}

/// Whether `target` installs the plugin.
#[must_use]
pub fn distribution(plugin: HasBin, target: DistributionTarget) -> Distribution {
    targets(target, plugin)
}

crate::registry::exports! {
    "loading/bin" {
        "binPathEntries" => async ok |plugin: HasBin, plugin_id: String, locations: Port|
            bin_path_entries(plugin, &plugin_id, &locations).await;
        "pathWithPluginBins" => ok |user_entries: Vec<String>, bin_directories: Vec<String>|
            path_with_plugin_bins(&user_entries, &bin_directories);
        "DISTRIBUTION_TARGETS" => ok || DISTRIBUTION_TARGETS;
        "distribution" => ok |plugin: HasBin, target: DistributionTarget| distribution(plugin, target);
    }
}
