//! Port of `src/marketplace/install-plans.ts`, switched.
//!
//! What the `/plugin` panel and the marketplace commands do, as plans a host executes (plugins/
//! install.md, "Manage installed plugins", "Uninstall a plugin the project enables", "Update plugins
//! now", "Manage marketplaces"; plugins/cli-reference.md, "plugin marketplace update"): pure
//! decisions over facts the host read; the kit moves no file and writes no settings.

use serde::{Deserialize, Serialize};

use super::install_scopes::{ChosenScope, InstallScope, is_writable_scope, scope_reach};

// ---- the installed plugin's details menu ----

/// What the details menu reads of an installed plugin.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionFacts {
    /// Whether it is on.
    pub enabled: bool,
    /// Every scope the plugin is installed or enabled in.
    pub scopes: Vec<InstallScope>,
    /// The manifest declares `userConfig` options.
    pub user_config: bool,
    /// The plugin includes a bundled MCP server (a `.mcpb`) with its own `user_config`.
    pub bundled_mcp_server: bool,
}

/// The items of a plugin's details menu: `Disable plugin` or `Enable plugin`, `Update now` and
/// `Uninstall`, then `Configure options` (a `userConfig`) and `Configure` (a bundled MCP server),
/// which "a plugin can show both". A plugin only at Managed scope "you can't enable, disable, or
/// uninstall here". Judgment (spec/judgments.md): it keeps `Update now`, since `claude plugin
/// update` accepts the managed scope, and a plugin that is also at a writable scope keeps its actions.
#[must_use]
pub fn plugin_actions(facts: &PluginActionFacts) -> Vec<String> {
    let changeable = facts.scopes.is_empty() || facts.scopes.iter().copied().any(is_writable_scope);
    let mut actions: Vec<String> = Vec::new();
    if changeable {
        actions.push(if facts.enabled {
            "Disable plugin".to_owned()
        } else {
            "Enable plugin".to_owned()
        });
    }
    actions.push("Update now".to_owned());
    if changeable {
        actions.push("Uninstall".to_owned());
    }
    if facts.user_config {
        actions.push("Configure options".to_owned());
    }
    if facts.bundled_mcp_server {
        actions.push("Configure".to_owned());
    }
    actions
}

/// What closing the panel does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "reload",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum PanelClose {
    /// `/reload-plugins` runs.
    Run,
    /// Nothing is pending.
    None,
    /// The reload waits for the response.
    QueueUntilResponseEnds,
    /// The changes stay pending.
    WarnAndLeavePending {
        /// The command that applies them anyway.
        force_command: String,
    },
}

/// What the panel's close reads.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelState {
    /// There are changes made in the panel.
    pub pending: bool,
    /// The reload would invalidate the prompt cache.
    pub invalidates_prompt_cache: bool,
    /// A response is streaming in the session.
    pub streaming: bool,
}

/// "When you close the `/plugin` panel with pending changes you made in it, Claude Code runs
/// `/reload-plugins` for you. If the reload would invalidate the prompt cache, it warns and leaves
/// the changes pending instead. Run `/reload-plugins --force` to apply them anyway." The panel "runs
/// `/reload-plugins` for you, or queues it until the response that's streaming finishes"
/// (troubleshooting.md, `Run /reload-plugins to apply.`). Judgment (spec/judgments.md): the warning
/// comes first, since the docs give the prompt-cache refusal without a streaming exception.
#[must_use]
pub fn close_panel(state: PanelState) -> PanelClose {
    if !state.pending {
        return PanelClose::None;
    }
    if state.invalidates_prompt_cache {
        return PanelClose::WarnAndLeavePending {
            force_command: "/reload-plugins --force".to_owned(),
        };
    }
    if state.streaming {
        PanelClose::QueueUntilResponseEnds
    } else {
        PanelClose::Run
    }
}

// ---- uninstalling a plugin the project enables ----

/// Writes the plugin's value into a settings file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Writes {
    /// The file.
    pub file: String,
    /// The value.
    pub value: bool,
}

/// One answer of the uninstall question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallOption {
    /// `y` or `u`.
    pub key: String,
    /// What it says.
    pub label: String,
    /// Writes the plugin's value into this settings file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writes: Option<Writes>,
    /// Removes the plugin from this settings file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removes_from: Option<String>,
    /// Whether the plugin stays installed for the project afterwards.
    pub installed: bool,
}

fn file_of(scope: ChosenScope) -> String {
    scope_reach(scope).file.to_owned()
}

/// What the uninstall question reads.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallFacts {
    /// The project's settings enable the plugin.
    pub enabled_by_project: bool,
}

/// For a plugin this repository's `.claude/settings.json` enables, Uninstall asks: `y`, **Disable
/// for me** writes `false` for it in your `.claude/settings.local.json` and leaves it installed for
/// the project; `u`, **Uninstall for everyone** removes it from the shared `.claude/settings.json`.
/// Any other plugin is uninstalled without the question (`None`).
#[must_use]
pub fn uninstall_choice(facts: UninstallFacts) -> Option<Vec<UninstallOption>> {
    if !facts.enabled_by_project {
        return None;
    }
    Some(vec![
        UninstallOption {
            key: "y".to_owned(),
            label: "Disable for me".to_owned(),
            writes: Some(Writes {
                file: file_of(ChosenScope::Local),
                value: false,
            }),
            removes_from: None,
            installed: true,
        },
        UninstallOption {
            key: "u".to_owned(),
            label: "Uninstall for everyone".to_owned(),
            writes: None,
            removes_from: Some(file_of(ChosenScope::Project)),
            installed: false,
        },
    ])
}

// ---- marketplaces ----

/// What a marketplace command does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketplaceAction {
    /// `list`.
    List,
    /// `update`.
    Update,
    /// `remove`.
    Remove,
}

/// Where the command is typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandSurface {
    /// A shell.
    Shell,
    /// A session.
    Session,
}

const fn prefix_of(surface: CommandSurface) -> &'static str {
    match surface {
        CommandSurface::Shell => "claude plugin marketplace",
        CommandSurface::Session => "/plugin marketplace",
    }
}

const fn action_word(action: MarketplaceAction) -> &'static str {
    match action {
        MarketplaceAction::List => "list",
        MarketplaceAction::Update => "update",
        MarketplaceAction::Remove => "remove",
    }
}

/// The command for an action, in a shell or in a session. `claude plugin marketplace remove <name>`
/// needs the name; `update` takes one or none; in a session `/plugin marketplace remove [name]`
/// opens the Marketplaces tab with none (cli-reference.md, "plugin marketplace remove", "Commands in
/// a session").
#[must_use]
pub fn marketplace_command(
    action: MarketplaceAction,
    surface: CommandSurface,
    name: Option<&str>,
) -> String {
    let mut parts = vec![prefix_of(surface), action_word(action)];
    parts.extend(name);
    parts.join(" ")
}

crate::registry::exports! {
    "marketplace/install-plans" {
        "pluginActions" => ok |facts: PluginActionFacts| plugin_actions(&facts);
        "closePanel" => ok |state: PanelState| close_panel(state);
        "uninstallChoice" => ok |facts: UninstallFacts| uninstall_choice(facts);
        "marketplaceCommand" => ok |action: MarketplaceAction, surface: CommandSurface, name: Option<String>| marketplace_command(action, surface, name.as_deref());
    }
}
