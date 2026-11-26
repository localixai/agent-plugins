//! Port of `src/marketplace/install-scopes.ts`, switched.
//!
//! The settings scopes a plugin can be enabled or installed in (plugins/host-marketplace.md,
//! "Migrate users with a renames map" and "Uninstall removed plugins from users' machines": "the
//! user, project, and local settings scopes"; managed settings are the one the kit and Claude Code
//! cannot write). A scope is a name only; reading the settings of each is the settings stream's.

use serde::{Deserialize, Serialize};

use crate::format::settings_files::SETTINGS_FILE;

/// The SDK's setting sources but `flag` (the `--settings` tier), which the docs do not name as a
/// scope a plugin is installed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallScope {
    /// `user`.
    User,
    /// `project`.
    Project,
    /// `local`.
    Local,
    /// `managed`.
    Managed,
}

/// Every scope, in the package's order.
pub const INSTALL_SCOPES: [InstallScope; 4] = [
    InstallScope::User,
    InstallScope::Project,
    InstallScope::Local,
    InstallScope::Managed,
];

/// Whether the user can have a plugin changed in the scope: every scope but `managed`.
#[must_use]
pub const fn is_writable_scope(scope: InstallScope) -> bool {
    match scope {
        InstallScope::User | InstallScope::Project | InstallScope::Local => true,
        InstallScope::Managed => false,
    }
}

/// Who a scope reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Audience {
    /// `you`.
    #[serde(rename = "you")]
    You,
    /// `everyone-in-repository`.
    #[serde(rename = "everyone-in-repository")]
    EveryoneInRepository,
}

/// In which projects a scope applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Reach {
    /// `every-project`.
    #[serde(rename = "every-project")]
    EveryProject,
    /// `this-repository`.
    #[serde(rename = "this-repository")]
    ThisRepository,
}

/// Who a scope reaches, and in which projects (plugins/install.md, "Choose an install scope";
/// plugins/overview.md, "Understand install scopes").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeReach {
    /// The settings file that records the plugin as enabled, as the docs write it.
    pub file: &'static str,
    /// Who it reaches.
    pub audience: Audience,
    /// In which projects.
    pub reach: Reach,
    /// The file is checked into the repository.
    pub committed: bool,
    /// Each collaborator still installs the plugin on their own machine.
    pub collaborators_install: bool,
}

/// Who `scope` reaches (the package's `REACH` table and `scopeReach`).
#[must_use]
pub const fn scope_reach(scope: ChosenScope) -> ScopeReach {
    match scope {
        ChosenScope::User => ScopeReach {
            file: SETTINGS_FILE.user,
            audience: Audience::You,
            reach: Reach::EveryProject,
            committed: false,
            collaborators_install: false,
        },
        ChosenScope::Project => ScopeReach {
            file: SETTINGS_FILE.project,
            audience: Audience::EveryoneInRepository,
            reach: Reach::ThisRepository,
            committed: true,
            collaborators_install: true,
        },
        ChosenScope::Local => ScopeReach {
            file: SETTINGS_FILE.local,
            audience: Audience::You,
            reach: Reach::ThisRepository,
            committed: false,
            collaborators_install: false,
        },
    }
}

/// The scopes a user chooses between in a command or the `/plugin` panel: the three settings
/// files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChosenScope {
    /// `user`.
    User,
    /// `project`.
    Project,
    /// `local`.
    Local,
}

/// The scopes from the one that wins to the one that loses, the one precedence table: "the local
/// setting overrides the project setting, and the project setting overrides the user setting", and
/// managed settings override every other file (settings-reference.md, `enabledPlugins`).
pub const SCOPES_BY_PRECEDENCE: [InstallScope; 4] = [
    InstallScope::Managed,
    InstallScope::Local,
    InstallScope::Project,
    InstallScope::User,
];

const fn is_chosen(scope: InstallScope) -> Option<ChosenScope> {
    match scope {
        InstallScope::User => Some(ChosenScope::User),
        InstallScope::Project => Some(ChosenScope::Project),
        InstallScope::Local => Some(ChosenScope::Local),
        InstallScope::Managed => None,
    }
}

/// The highest-precedence scope among `scopes` that has a settings file; `managed` has none the
/// user writes.
#[must_use]
pub fn most_specific_scope(scopes: &[InstallScope]) -> Option<ChosenScope> {
    SCOPES_BY_PRECEDENCE
        .into_iter()
        .filter(|scope| scopes.contains(scope))
        .find_map(is_chosen)
}

/// The shell command that installs a plugin: `claude plugin install <plugin>`.
#[must_use]
pub fn install_command(plugin: &str) -> String {
    format!("claude plugin install {plugin}")
}

/// What a collaborator runs once after the project's `.claude/settings.json` enables the plugin.
#[must_use]
pub fn collaborator_install_command(plugin_id: &str) -> String {
    format!("{} --scope project", install_command(plugin_id))
}

/// The shell commands that take `--scope` and default when it is omitted (plugins/install.md,
/// "Manage plugins from your shell").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopedCommand {
    /// `install`.
    Install,
    /// `uninstall`.
    Uninstall,
    /// `enable`.
    Enable,
    /// `disable`.
    Disable,
}

/// The scope a command acts on when `--scope` is omitted: `install` and `uninstall` use user
/// scope; `enable` and `disable` use "the most specific scope whose settings already list the
/// plugin" (`listed_in`), none when no settings file lists it.
#[must_use]
pub fn default_scope(command: ScopedCommand, listed_in: &[InstallScope]) -> Option<ChosenScope> {
    match command {
        ScopedCommand::Install | ScopedCommand::Uninstall => Some(ChosenScope::User),
        ScopedCommand::Enable | ScopedCommand::Disable => most_specific_scope(listed_in),
    }
}

crate::registry::exports! {
    "marketplace/install-scopes" {
        "INSTALL_SCOPES" => ok || INSTALL_SCOPES;
        "isWritableScope" => ok |scope: InstallScope| is_writable_scope(scope);
        "scopeReach" => ok |scope: ChosenScope| scope_reach(scope);
        "SCOPES_BY_PRECEDENCE" => ok || SCOPES_BY_PRECEDENCE;
        "mostSpecificScope" => ok |scopes: Vec<InstallScope>| most_specific_scope(&scopes);
        "installCommand" => ok |plugin: String| install_command(&plugin);
        "collaboratorInstallCommand" => ok |plugin_id: String| collaborator_install_command(&plugin_id);
        "defaultScope" => ok |command: ScopedCommand, listed_in: Vec<InstallScope>| default_scope(command, &listed_in);
    }
}
