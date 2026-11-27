//! Port of `src/marketplace/install-commands.ts`, switched.
//!
//! What an install command does on each surface (plugins/install.md, "Install a plugin", "Install
//! from your shell", "Add a marketplace and install in one command", "JetBrains, non-interactive
//! runs, and the Agent SDK"): pure plans of what the command opens, adds, asks and prints; the host
//! runs them. The kit adds nothing to settings and prints nothing itself.

use serde::{Deserialize, Serialize, Serializer};

use super::install_errors::{InstallSurface, PLUGIN_ACTIVE, RELOAD_TO_APPLY};
use super::install_hosts::OFFICIAL_MARKETPLACE_ADD;
use super::install_scopes::{ChosenScope, ScopedCommand, default_scope};
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsValue, is_whitespace};

/// An install command as typed.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallCommand {
    /// Where it was typed.
    pub surface: InstallSurface,
    /// `<name>@<marketplace>`, or the bare name when `marketplace` (the `--marketplace` source) is given.
    pub plugin: String,
    /// The `--marketplace` source: "the same forms as `/plugin marketplace add`".
    #[serde(default)]
    pub marketplace: Option<String>,
    /// `--scope` in a shell.
    #[serde(default)]
    pub scope: Option<ChosenScope>,
    /// A marketplace from that source is already added.
    #[serde(default)]
    pub marketplace_added: Option<bool>,
}

/// Where a marketplace the shell adds is declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeclaredIn {
    /// The user settings.
    User,
}

/// The key is always there: `undefined` where the TypeScript writes `marketplaceDeclaredIn:
/// undefined` or `scope: undefined`.
fn or_undefined<T: Serialize, S: Serializer>(
    value: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => value.serialize(serializer),
        None => JsValue::Undefined.serialize(serializer),
    }
}

/// What the command does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the members of the TypeScript's `InstallCommandPlan`, one yes or no each"
)]
pub struct InstallCommandPlan {
    /// The plugin is installed now; false when the command only opens its details to review.
    pub installs: bool,
    /// "In a session, this command doesn't install right away: it opens the `/plugin` panel on that plugin's details."
    pub opens_details: bool,
    /// "Claude Code shows the source it resolved and asks you to confirm before adding it."
    pub confirms_marketplace: bool,
    /// A marketplace the shell adds "is declared in your user settings even when you pass `--scope project`".
    #[serde(serialize_with = "or_undefined")]
    pub marketplace_declared_in: Option<DeclaredIn>,
    /// The scope of the install; `undefined` in a session, where the panel asks.
    #[serde(serialize_with = "or_undefined")]
    pub scope: Option<ChosenScope>,
    /// A marketplace is added.
    pub adds_marketplace: bool,
}

/// The plan, or the refusal.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum PlannedInstall {
    /// The plan.
    Plan(InstallCommandPlan),
    /// The refusal.
    Refused(PluginIssue),
}

/// What each surface does (`SURFACES`); `addsMarketplace` is set by the caller.
fn surfaces(surface: InstallSurface, command: &InstallCommand, adds: bool) -> InstallCommandPlan {
    match surface {
        InstallSurface::Session => InstallCommandPlan {
            installs: false,
            opens_details: true,
            confirms_marketplace: adds,
            marketplace_declared_in: None,
            scope: None,
            adds_marketplace: adds,
        },
        InstallSurface::Shell => InstallCommandPlan {
            installs: true,
            opens_details: false,
            confirms_marketplace: false,
            marketplace_declared_in: adds.then_some(DeclaredIn::User),
            scope: command
                .scope
                .or_else(|| default_scope(ScopedCommand::Install, &[])),
            adds_marketplace: adds,
        },
    }
}

fn refuse(message: &str) -> PluginIssue {
    error("plugin", message)
}

/// The refusals before anything runs: a suffixed name beside `--marketplace`, a source with spaces in a session.
fn command_issue(command: &InstallCommand) -> Option<PluginIssue> {
    let marketplace = command.marketplace.as_deref()?;
    if command.plugin.contains('@') {
        return Some(refuse(&format!(
            "Give the plugin name by itself, without an @marketplace suffix, when --marketplace names the source: {}",
            command.plugin
        )));
    }
    (command.surface == InstallSurface::Session && marketplace.chars().any(is_whitespace)).then(
        || {
            refuse(&format!(
                "In a session, the --marketplace source can't contain spaces: {marketplace}"
            ))
        },
    )
}

/// Plans an install command. A marketplace "you've already added from that source is reused";
/// a new one is added, after a confirmation in a session and without one from a shell.
#[must_use]
pub fn plan_install_command(command: &InstallCommand) -> PlannedInstall {
    if let Some(refused) = command_issue(command) {
        return PlannedInstall::Refused(refused);
    }
    let adds = command.marketplace.is_some() && command.marketplace_added != Some(true);
    PlannedInstall::Plan(surfaces(command.surface, command, adds))
}

/// What `shellInstallLines` is given.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallLinesInput {
    /// The plugin.
    pub plugin_id: String,
    /// The scope.
    pub scope: ChosenScope,
    /// The name of the marketplace the command added; absent when it reused one or none was named.
    #[serde(default)]
    pub added_marketplace: Option<String>,
}

/// The scope as the shell prints it.
const fn scope_text(scope: ChosenScope) -> &'static str {
    match scope {
        ChosenScope::User => "user",
        ChosenScope::Project => "project",
        ChosenScope::Local => "local",
    }
}

/// What the shell prints, in order: the marketplace it added, then the install ("when it finishes").
#[must_use]
pub fn shell_install_lines(input: &InstallLinesInput) -> Vec<String> {
    let mut lines: Vec<String> = input
        .added_marketplace
        .iter()
        .map(|added| format!("Successfully added marketplace: {added} (declared in user settings)"))
        .collect();
    lines.push(format!(
        "Successfully installed plugin: {} (scope: {})",
        input.plugin_id,
        scope_text(input.scope)
    ));
    lines
}

/// What `shellInstallPrelude` is given.
#[derive(Clone, Debug, Deserialize)]
pub struct PreludeInput {
    /// The marketplace the install names.
    pub marketplace: String,
    /// The marketplaces registered.
    pub registered: Vec<String>,
}

/// "On a machine where no one has opened an interactive Claude Code session yet, the official
/// marketplace isn't registered, so a script that installs from it runs `claude plugin marketplace
/// add anthropics/claude-plugins-official` before the install."
#[must_use]
pub fn shell_install_prelude(input: &PreludeInput) -> Vec<String> {
    let needed = input.marketplace == OFFICIAL_MARKETPLACE_ADD.name
        && !input
            .registered
            .iter()
            .any(|name| name == OFFICIAL_MARKETPLACE_ADD.name);
    if needed {
        vec![format!(
            "claude plugin marketplace add {}",
            OFFICIAL_MARKETPLACE_ADD.source
        )]
    } else {
        vec![]
    }
}

// ---- the install summary ----

/// What the summary depends on.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the facts the TypeScript's summary reads, one yes or no each"
)]
pub struct SummaryFacts {
    /// Activating the plugin now would invalidate the prompt cache.
    pub invalidates_prompt_cache: bool,
    /// Activation failed.
    pub activation_failed: bool,
    /// The plugin could not be loaded at all.
    pub load_failed: bool,
    /// The plugin's bundled MCP server needs options before it can start.
    pub bundled_server_needs_setup: bool,
}

/// The summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSummary {
    /// The last sentences of the summary.
    pub lines: Vec<String>,
    /// The panel closes and Claude Code runs `/reload-plugins` for you.
    pub runs_reload: bool,
    /// The reload would invalidate the prompt cache: it warns and leaves the plugin pending.
    pub pending: bool,
}

const SETUP: &str = "Its bundled MCP server needs configuration before it can start";
const LOAD_FAILED: &str = "The plugin couldn't be loaded";

/// The summary after the scope is chosen: "Active now" (`Plugin is now active.`), active with a
/// server that needs setup, "Reload needed" (`Run /reload-plugins to apply.`, which the panel runs
/// for you unless it would invalidate the prompt cache, when the plugin stays pending until
/// `/reload-plugins --force`), or "Load failed".
#[must_use]
pub fn install_summary(facts: &SummaryFacts) -> InstallSummary {
    if facts.load_failed {
        return InstallSummary {
            lines: vec![LOAD_FAILED.to_owned()],
            runs_reload: false,
            pending: false,
        };
    }
    if facts.activation_failed || facts.invalidates_prompt_cache {
        return InstallSummary {
            lines: vec![RELOAD_TO_APPLY.to_owned()],
            runs_reload: !facts.invalidates_prompt_cache,
            pending: facts.invalidates_prompt_cache,
        };
    }
    let mut lines = vec![PLUGIN_ACTIVE.to_owned()];
    if facts.bundled_server_needs_setup {
        lines.push(SETUP.to_owned());
    }
    InstallSummary {
        lines,
        runs_reload: false,
        pending: false,
    }
}

crate::registry::exports! {
    "marketplace/install-commands" {
        "planInstallCommand" => ok |command: InstallCommand| plan_install_command(&command);
        "shellInstallLines" => ok |input: InstallLinesInput| shell_install_lines(&input);
        "shellInstallPrelude" => ok |input: PreludeInput| shell_install_prelude(&input);
        "installSummary" => ok |facts: SummaryFacts| install_summary(&facts);
    }
}
