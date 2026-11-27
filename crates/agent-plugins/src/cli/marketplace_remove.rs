//! Port of `src/cli/marketplace-remove.ts`, switched.
//!
//! `claude plugin marketplace remove <name>` as a plan (plugins/cli-reference.md, "plugin marketplace
//! remove"): which settings files lose the declaration and, when the last one does, the cache, the
//! plugins installed from it and their options, secrets and data. Pure over what the host read; the
//! host deletes. `--scope` has no `-s`. From the last scope it also "removes their `enabledPlugins`
//! entries from your settings files" (plugins/install.md, "Remove a marketplace"): "you installed" and
//! "your settings files" are why `managed` is left alone. The Marketplaces tab names the `uninstall`
//! ids before it asks to confirm. The wording for a marketplace nobody knows and for one plugin, and
//! what a plugin only managed settings installed does, are in spec/judgments.md.

use serde::{Deserialize, Serialize};

use super::marketplace_add::{RESERVED_QUIET_NAME, is_reserved_name};
use super::result::{
    CommandOutput, MarketplaceCommand, marketplace_failure, marketplace_success, no_result_line,
};
use super::scope::{ChosenOrError, ScopeChoice, ScopedCommand, choose_scope};
use crate::marketplace::install_scopes::{ChosenScope, InstallScope, is_writable_scope};

const REMOVE: MarketplaceCommand = MarketplaceCommand::MarketplaceRemove;

/// The command line.
#[derive(Clone, Debug, Deserialize)]
pub struct RemoveArguments {
    /// The marketplace name `plugin marketplace list` shows, not the source passed to `add`.
    pub name: String,
    /// `--scope`.
    #[serde(default)]
    pub scope: Option<String>,
}

/// A plugin installed from the marketplace and the scopes it is installed at.
#[derive(Clone, Debug, Deserialize)]
pub struct InstalledFromMarketplace {
    /// `name@marketplace`.
    pub id: String,
    /// Its scopes.
    pub scopes: Vec<InstallScope>,
}

/// One `enabledPlugins` key in one settings file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnabledEntry {
    /// `name@marketplace`.
    pub id: String,
    /// The settings file.
    pub scope: ChosenScope,
}

/// What the host read.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveFacts {
    /// The marketplace is registered (on disk or declared).
    pub known: bool,
    /// The settings files that declare it.
    pub declared_in: Vec<InstallScope>,
    /// The plugins installed from it.
    pub plugins: Vec<InstalledFromMarketplace>,
    /// Every `enabledPlugins` entry in the user, project and local settings files.
    pub enabled_entries: Vec<EnabledEntry>,
    /// `CLAUDE_CODE_PLUGIN_SEED_DIR` supplies it: the seed directory; it is read-only.
    #[serde(default)]
    pub seeded: Option<String>,
}

/// One plugin uninstalled because its marketplace is gone: "Also deletes their saved options and
/// secrets and data where it can". The flags are the docs' three deletions, every one on because the
/// plugin leaves its last scope; `--keep-data` and the cases that keep the data directory belong to
/// `plugin uninstall`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRemoval {
    /// The plugin.
    pub id: String,
    /// The scopes to uninstall from: every scope but `managed`, which the user cannot change.
    pub scopes: Vec<InstallScope>,
    /// Always `true`.
    pub delete_options: bool,
    /// Always `true`.
    pub delete_secrets: bool,
    /// Always `true`.
    pub delete_data: bool,
}

/// What the command does and prints.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovePlan {
    /// What it prints.
    pub output: CommandOutput,
    /// The settings files to remove the declaration from.
    pub remove_from: Vec<InstallScope>,
    /// The marketplace was removed from the last scope that declares it.
    pub delete_cache: bool,
    /// The plugins uninstalled.
    pub uninstall: Vec<PluginRemoval>,
    /// The `enabledPlugins` entries of the marketplace's plugins to delete, when it leaves its last scope.
    pub drop_enabled_entries: Vec<EnabledEntry>,
}

fn removal(plugin: &InstalledFromMarketplace) -> PluginRemoval {
    PluginRemoval {
        id: plugin.id.clone(),
        scopes: plugin
            .scopes
            .iter()
            .copied()
            .filter(|scope| is_writable_scope(*scope))
            .collect(),
        delete_options: true,
        delete_secrets: true,
        delete_data: true,
    }
}

const fn nothing(output: CommandOutput) -> RemovePlan {
    RemovePlan {
        output,
        remove_from: Vec::new(),
        delete_cache: false,
        uninstall: Vec::new(),
        drop_enabled_entries: Vec::new(),
    }
}

fn line_of(name: &str, uninstalled: &[PluginRemoval]) -> String {
    let head = format!("Successfully removed marketplace: {name}");
    if uninstalled.is_empty() {
        return head;
    }
    let noun = if uninstalled.len() == 1 {
        "plugin"
    } else {
        "plugins"
    };
    let mut lines = vec![
        head,
        format!(
            "Also uninstalled {} {noun} from this marketplace:",
            uninstalled.len()
        ),
    ];
    lines.extend(uninstalled.iter().map(|one| format!("  {}", one.id)));
    lines.join("\n")
}

fn success(name: &str, uninstalled: &[PluginRemoval]) -> CommandOutput {
    marketplace_success(REMOVE, Some(name), &line_of(name, uninstalled))
}

/// The declarations that go: the one `--scope` names, or every one without it.
fn declarations_to_remove(chosen: Option<InstallScope>, facts: &RemoveFacts) -> Vec<InstallScope> {
    match chosen {
        Some(scope) => facts
            .declared_in
            .iter()
            .copied()
            .filter(|declared| *declared == scope)
            .collect(),
        None => facts.declared_in.clone(),
    }
}

/// "`remove` without `--scope` on a seed marketplace fail[s] with a message that names the seed
/// directory" (plugins/org.md, "Seed containers and CI"). Judgment: the wording follows `marketplace
/// update`'s and the code is its `seed_managed` (spec/judgments.md).
fn seed_managed(name: &str, seed: &str) -> RemovePlan {
    let message = format!(
        "Marketplace '{name}' is seed-managed ({seed}). Ask your administrator to remove it from the seed directory, or remove it from one scope with --scope"
    );
    nothing(marketplace_failure(
        REMOVE,
        Some(name),
        &message,
        "seed_managed",
    ))
}

/// What `plugin marketplace remove` does and prints.
#[must_use]
pub fn plan_marketplace_remove(args: &RemoveArguments, facts: &RemoveFacts) -> RemovePlan {
    if is_reserved_name(&args.name) {
        let reason = format!("\"{RESERVED_QUIET_NAME}\" is a reserved marketplace name");
        return nothing(no_result_line(&reason));
    }
    let chosen = match choose_scope(ScopedCommand::MarketplaceRemove, args.scope.as_deref()) {
        ChosenOrError::Error(output) => return nothing(output),
        ChosenOrError::Choice(ScopeChoice::Scope { scope }) => Some(scope),
        ChosenOrError::Choice(ScopeChoice::AutoDetect { .. } | ScopeChoice::AllScopes { .. }) => {
            None
        }
    };
    let name = &args.name;
    if !facts.known {
        let message = format!("Marketplace '{name}' not found");
        return nothing(marketplace_failure(
            REMOVE,
            Some(name),
            &message,
            "not_found",
        ));
    }
    if let (Some(seed), None) = (&facts.seeded, chosen) {
        return seed_managed(name, seed);
    }
    let remove_from = declarations_to_remove(chosen, facts);
    if let Some(scope) = chosen.filter(|_| remove_from.is_empty()) {
        let message = format!(
            "Marketplace '{name}' is not declared in {} settings. Omit --scope to remove it from all scopes.",
            scope.as_str()
        );
        return nothing(marketplace_failure(
            REMOVE,
            Some(name),
            &message,
            "not_declared_in_scope",
        ));
    }
    let last = facts
        .declared_in
        .iter()
        .all(|scope| remove_from.contains(scope));
    if !last {
        return RemovePlan {
            remove_from,
            ..nothing(success(name, &[]))
        };
    }
    let uninstall: Vec<PluginRemoval> = facts
        .plugins
        .iter()
        .map(removal)
        .filter(|one| !one.scopes.is_empty())
        .collect();
    let suffix = format!("@{name}");
    RemovePlan {
        output: success(name, &uninstall),
        remove_from,
        delete_cache: true,
        uninstall,
        drop_enabled_entries: facts
            .enabled_entries
            .iter()
            .filter(|one| one.id.ends_with(&suffix))
            .cloned()
            .collect(),
    }
}

crate::registry::exports! {
    "cli/marketplace-remove" {
        "planMarketplaceRemove" => ok |args: RemoveArguments, facts: RemoveFacts| plan_marketplace_remove(&args, &facts);
    }
}
