//! Port of `src/marketplace/install-hosts.ts`, switched.
//!
//! Where a plugin is installed from and what each place does (plugins/install.md, "Install a
//! plugin" and its tabs, "Install from your shell", "JetBrains, non-interactive runs, and the Agent
//! SDK"): one table of facts per host, read only through the plans below, which a host runs. The
//! kit opens nothing and adds nothing itself.

use serde::{Deserialize, Serialize};

use crate::marketplace::install_scopes::ChosenScope;

/// Where a plugin is installed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallHost {
    /// `terminal`.
    #[serde(rename = "terminal")]
    Terminal,
    /// `jetbrains`.
    #[serde(rename = "jetbrains")]
    Jetbrains,
    /// `desktop`.
    #[serde(rename = "desktop")]
    Desktop,
    /// `vscode`.
    #[serde(rename = "vscode")]
    Vscode,
    /// `non-interactive`.
    #[serde(rename = "non-interactive")]
    NonInteractive,
    /// `agent-sdk`.
    #[serde(rename = "agent-sdk")]
    AgentSdk,
    /// `cloud`.
    #[serde(rename = "cloud")]
    Cloud,
}

/// What to use to install and manage plugins on a host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallWith {
    /// `panel`.
    #[serde(rename = "panel")]
    Panel,
    /// `terminal-steps`.
    #[serde(rename = "terminal-steps")]
    TerminalSteps,
    /// `plugin-browser`.
    #[serde(rename = "plugin-browser")]
    PluginBrowser,
    /// `manage-plugins`.
    #[serde(rename = "manage-plugins")]
    ManagePlugins,
    /// `shell-commands`.
    #[serde(rename = "shell-commands")]
    ShellCommands,
    /// `sdk-plugins-option`.
    #[serde(rename = "sdk-plugins-option")]
    SdkPluginsOption,
    /// `none`.
    #[serde(rename = "none")]
    None,
}

/// When an install takes effect in a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplyMoment {
    /// `next-start`.
    #[serde(rename = "next-start")]
    NextStart,
    /// `reload-plugins`.
    #[serde(rename = "reload-plugins")]
    ReloadPlugins,
    /// `install-summary`.
    #[serde(rename = "install-summary")]
    InstallSummary,
    /// `open-sessions`.
    #[serde(rename = "open-sessions")]
    OpenSessions,
}

struct HostFacts {
    /// `/plugin` runs here and opens the panel.
    plugin_panel: bool,
    install_with: InstallWith,
    /// The reply to `/plugin` where it does not run.
    panel_reply: Option<&'static str>,
    /// The scopes the install surface offers the user to choose between; none where there is no surface.
    scopes: &'static [ChosenScope],
    /// Claude Code starts an interactive terminal session here, which registers the official marketplace the first time.
    starts_terminal_session: bool,
    /// When a plugin installed in this host's own surface takes effect; `None` where the docs say nothing.
    applies_after_install: Option<&'static [ApplyMoment]>,
    /// Plugins already installed on the machine, or enabled by the repository's settings, load.
    loads_installed: Option<bool>,
}

const THREE_SCOPES: &[ChosenScope] = &[ChosenScope::User, ChosenScope::Project, ChosenScope::Local];

const TERMINAL: HostFacts = HostFacts {
    plugin_panel: true,
    install_with: InstallWith::Panel,
    panel_reply: None,
    scopes: THREE_SCOPES,
    starts_terminal_session: true,
    // "prints an install summary": `installSummary` says whether the plugin is active or needs a reload
    applies_after_install: Some(&[ApplyMoment::InstallSummary]),
    loads_installed: Some(true),
};

const fn install_hosts(host: InstallHost) -> HostFacts {
    match host {
        InstallHost::Terminal => TERMINAL,
        // "the JetBrains plugin runs Claude Code in the IDE's terminal, so use the Terminal tab's steps there"
        InstallHost::Jetbrains => HostFacts {
            install_with: InstallWith::TerminalSteps,
            ..TERMINAL
        },
        // "Click the + button next to the prompt box and select Add plugins"
        InstallHost::Desktop => HostFacts {
            plugin_panel: false,
            install_with: InstallWith::PluginBrowser,
            panel_reply: None,
            scopes: THREE_SCOPES,
            starts_terminal_session: false,
            applies_after_install: None,
            loads_installed: Some(true),
        },
        // "Type /plugins ... Your changes apply to open sessions without a restart."
        InstallHost::Vscode => HostFacts {
            plugin_panel: false,
            install_with: InstallWith::ManagePlugins,
            panel_reply: None,
            scopes: THREE_SCOPES,
            starts_terminal_session: false,
            applies_after_install: Some(&[ApplyMoment::OpenSessions]),
            loads_installed: Some(true),
        },
        // "`/plugin` doesn't run ... Plugins you already installed do load."
        InstallHost::NonInteractive => HostFacts {
            plugin_panel: false,
            install_with: InstallWith::ShellCommands,
            panel_reply: Some("/plugin isn't available in this environment."),
            scopes: &[],
            starts_terminal_session: false,
            applies_after_install: None,
            loads_installed: Some(true),
        },
        InstallHost::AgentSdk => HostFacts {
            plugin_panel: false,
            install_with: InstallWith::SdkPluginsOption,
            panel_reply: None,
            scopes: &[],
            starts_terminal_session: false,
            applies_after_install: None,
            loads_installed: None,
        },
        // "has no plugin browser and doesn't load the plugins you installed on your own machine or the
        // ones your repository's `.claude/settings.json` turns on"
        InstallHost::Cloud => HostFacts {
            plugin_panel: false,
            install_with: InstallWith::None,
            panel_reply: None,
            scopes: &[],
            starts_terminal_session: false,
            applies_after_install: None,
            loads_installed: Some(false),
        },
    }
}

/// What `/plugin` opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginOpens {
    /// The Discover tab.
    Discover,
    /// The plugin's details.
    Details,
}

/// What `/plugin` does on a host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PluginCommandPlan {
    /// What `/plugin` opens: the Discover tab with no plugin name, the plugin's details with one; nothing where the panel does not run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opens: Option<PluginOpens>,
    /// What Claude replies where `/plugin` does not run, when the docs quote it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<String>,
}

/// What `planPluginCommand` takes.
#[derive(Clone, Debug, Deserialize)]
pub struct PluginCommandInput {
    /// The host.
    pub host: InstallHost,
    /// The plugin named, if any.
    #[serde(default)]
    pub plugin: Option<String>,
}

/// What `/plugin` does on a host: "run `/plugin` with no plugin name: the panel opens on the
/// Discover tab"; with a name it opens that plugin's details; where it does not run (`claude -p`)
/// Claude replies `/plugin isn't available in this environment.`
#[must_use]
pub fn plan_plugin_command(input: &PluginCommandInput) -> PluginCommandPlan {
    let facts = install_hosts(input.host);
    if !facts.plugin_panel {
        return PluginCommandPlan {
            opens: None,
            reply: facts.panel_reply.map(str::to_owned),
        };
    }
    PluginCommandPlan {
        opens: Some(if input.plugin.is_none() {
            PluginOpens::Discover
        } else {
            PluginOpens::Details
        }),
        reply: None,
    }
}

/// What to use to install and manage plugins on the host.
#[must_use]
pub const fn install_with(host: InstallHost) -> InstallWith {
    install_hosts(host).install_with
}

/// The scopes the host's install surface offers, in the order the docs list them; none without a surface.
#[must_use]
pub const fn scope_choices(host: InstallHost) -> &'static [ChosenScope] {
    install_hosts(host).scopes
}

/// Whether plugins installed on the machine, or enabled by the repository, load on the host; `None` where the docs say nothing.
#[must_use]
pub const fn loads_installed_plugins(host: InstallHost) -> Option<bool> {
    install_hosts(host).loads_installed
}

/// `claude plugin install` in a shell: "plugins it installs load the next time you start Claude
/// Code, or when you run `/reload-plugins` in a session that's already open".
const SHELL_APPLIES: &[ApplyMoment] = &[ApplyMoment::NextStart, ApplyMoment::ReloadPlugins];

/// Where an install was made: `claude plugin install` in a shell, or the host's own install surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplyOrigin {
    /// `claude plugin install` in a shell.
    Shell,
    /// The host's own install surface.
    Surface,
}

/// When an install takes effect: from a shell, at the next start or on `/reload-plugins`, whatever
/// the host; from the host's own surface, as that surface says (VS Code "applies to open
/// sessions without a restart"); `None` where the docs say nothing.
#[must_use]
pub const fn plan_plugin_apply(
    host: InstallHost,
    origin: ApplyOrigin,
) -> Option<&'static [ApplyMoment]> {
    match origin {
        ApplyOrigin::Shell => Some(SHELL_APPLIES),
        ApplyOrigin::Surface => install_hosts(host).applies_after_install,
    }
}

/// A marketplace Claude Code adds on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct MarketplaceAdd {
    /// Its name.
    pub name: &'static str,
    /// Its source.
    pub source: &'static str,
}

/// Anthropic's official marketplace.
pub const OFFICIAL_MARKETPLACE_ADD: MarketplaceAdd = MarketplaceAdd {
    name: "claude-plugins-official",
    source: "anthropics/claude-plugins-official",
};

/// What `marketplacesAddedAtStart` takes.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplacesAddedAtStartInput {
    /// The host.
    pub host: InstallHost,
    /// No interactive terminal session has run on this machine yet.
    pub first_interactive_start: bool,
    /// The marketplaces already registered.
    pub registered: Vec<String>,
}

/// The marketplaces Claude Code adds on its own at start: "Claude Code adds Anthropic's official
/// marketplace for you the first time you start an interactive terminal session". Nothing is added
/// by any other start (a shell command, `claude -p`, another surface) or once it is registered.
#[must_use]
pub fn marketplaces_added_at_start(input: &MarketplacesAddedAtStartInput) -> Vec<MarketplaceAdd> {
    let adds = install_hosts(input.host).starts_terminal_session
        && input.first_interactive_start
        && !input
            .registered
            .iter()
            .any(|name| name == OFFICIAL_MARKETPLACE_ADD.name);
    if adds {
        vec![OFFICIAL_MARKETPLACE_ADD]
    } else {
        Vec::new()
    }
}

crate::registry::exports! {
    "marketplace/install-hosts" {
        "planPluginCommand" => ok |input: PluginCommandInput| plan_plugin_command(&input);
        "installWith" => ok |host: InstallHost| install_with(host);
        "scopeChoices" => ok |host: InstallHost| scope_choices(host);
        "loadsInstalledPlugins" => ok |host: InstallHost| loads_installed_plugins(host);
        "planPluginApply" => ok |host: InstallHost, origin: ApplyOrigin| plan_plugin_apply(host, origin);
        "marketplacesAddedAtStart" => ok |input: MarketplacesAddedAtStartInput| marketplaces_added_at_start(&input);
    }
}
