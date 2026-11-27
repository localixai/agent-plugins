//! Port of `src/cli/scope.ts`, switched.
//!
//! The scope grammar of the plugin commands (plugins/cli-reference.md, "claude plugin commands":
//! "`--scope` takes `user`, `project`, or `local`, and names the settings file the command writes
//! to. `update` also takes `managed`"; each command's `--scope` row; "plugin enable"; "plugin
//! update", "Which scope the command updates"; "claude plugin marketplace commands": "their `--scope`
//! flag has no `-s` short form"). Pure: which scope a command works on from the flag and the scopes a
//! plugin is declared in; the host reads and writes the settings files.

use serde::{Deserialize, Serialize};

use super::result::{CommandOutput, usage_error};
use crate::marketplace::install_scopes::InstallScope;
use crate::settings::layers::{PolicyScope, rank_of};

/// `update` also takes `managed`, the one scope that can be updated but not installed to.
pub type CommandScope = InstallScope;

/// The values `--scope` takes on every command that takes it.
pub const CONFIG_SCOPES: [InstallScope; 3] = [
    InstallScope::User,
    InstallScope::Project,
    InstallScope::Local,
];

/// The commands that take `--scope`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScopedCommand {
    /// `install`.
    Install,
    /// `uninstall`.
    Uninstall,
    /// `enable`.
    Enable,
    /// `disable`.
    Disable,
    /// `update`.
    Update,
    /// `prune`.
    Prune,
    /// `marketplace-add`.
    MarketplaceAdd,
    /// `marketplace-remove`.
    MarketplaceRemove,
}

/// The commands that take `--scope`, in the package's order.
pub const SCOPED_COMMANDS: [ScopedCommand; 8] = [
    ScopedCommand::Install,
    ScopedCommand::Uninstall,
    ScopedCommand::Enable,
    ScopedCommand::Disable,
    ScopedCommand::Update,
    ScopedCommand::Prune,
    ScopedCommand::MarketplaceAdd,
    ScopedCommand::MarketplaceRemove,
];

/// What a command does when `--scope` is omitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ScopeChoice {
    /// That scope.
    Scope {
        /// The scope.
        scope: CommandScope,
    },
    /// The scope is found from where the plugin is declared (`detectScope`).
    AutoDetect {
        /// Always `true`.
        #[serde(rename = "autoDetect")]
        auto_detect: bool,
    },
    /// The command acts on every scope.
    AllScopes {
        /// Always `true`.
        #[serde(rename = "allScopes")]
        all_scopes: bool,
    },
}

struct ScopeRule {
    accepts: &'static [CommandScope],
    when_omitted: ScopeChoice,
    /// The scopes auto-detect checks, in order; empty for a command that does not detect.
    detects: &'static [CommandScope],
    /// The marketplace commands' `--scope` has no `-s`.
    short: bool,
}

const DEFAULT_USER: ScopeChoice = ScopeChoice::Scope {
    scope: InstallScope::User,
};
const AUTO: ScopeChoice = ScopeChoice::AutoDetect { auto_detect: true };
const LOCAL_PROJECT_USER: [CommandScope; 3] = [
    InstallScope::Local,
    InstallScope::Project,
    InstallScope::User,
];
const UPDATE_ACCEPTS: [CommandScope; 4] = [
    InstallScope::User,
    InstallScope::Project,
    InstallScope::Local,
    InstallScope::Managed,
];
const UPDATE_DETECTS: [CommandScope; 4] = [
    InstallScope::Local,
    InstallScope::Project,
    InstallScope::User,
    InstallScope::Managed,
];

/// The registry: a command added to `SCOPED_COMMANDS` fails the build until it has a rule.
const fn scope_rule(command: ScopedCommand) -> ScopeRule {
    match command {
        ScopedCommand::Install | ScopedCommand::Uninstall | ScopedCommand::Prune => ScopeRule {
            accepts: &CONFIG_SCOPES,
            when_omitted: DEFAULT_USER,
            detects: &[],
            short: true,
        },
        ScopedCommand::Enable | ScopedCommand::Disable => ScopeRule {
            accepts: &CONFIG_SCOPES,
            when_omitted: AUTO,
            detects: &LOCAL_PROJECT_USER,
            short: true,
        },
        ScopedCommand::Update => ScopeRule {
            accepts: &UPDATE_ACCEPTS,
            when_omitted: AUTO,
            detects: &UPDATE_DETECTS,
            short: true,
        },
        ScopedCommand::MarketplaceAdd => ScopeRule {
            accepts: &CONFIG_SCOPES,
            when_omitted: DEFAULT_USER,
            detects: &[],
            short: false,
        },
        ScopedCommand::MarketplaceRemove => ScopeRule {
            accepts: &CONFIG_SCOPES,
            when_omitted: ScopeChoice::AllScopes { all_scopes: true },
            detects: &[],
            short: false,
        },
    }
}

/// How the help spells the flag: `-s, --scope <scope>`, or `--scope <scope>` for a marketplace command.
#[must_use]
pub const fn scope_flag_spelling(command: ScopedCommand) -> &'static str {
    if scope_rule(command).short {
        "-s, --scope <scope>"
    } else {
        "--scope <scope>"
    }
}

fn accepted_scope(command: ScopedCommand, value: &str) -> Option<CommandScope> {
    scope_rule(command)
        .accepts
        .iter()
        .copied()
        .find(|scope| scope.as_str() == value)
}

/// A scope choice, or the usage error of a value the command does not take.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ChosenOrError {
    /// The choice.
    Choice(ScopeChoice),
    /// The usage error.
    Error(CommandOutput),
}

/// The scope `command` works on: the flag's value, or what the command does without it. A value the
/// command does not take is a usage error ("An invalid `--scope`": no result line, exit `1`, the
/// reason on stderr; the wording is the kit's, spec/judgments.md).
#[must_use]
pub fn choose_scope(command: ScopedCommand, given: Option<&str>) -> ChosenOrError {
    let Some(given) = given else {
        return ChosenOrError::Choice(scope_rule(command).when_omitted);
    };
    if let Some(scope) = accepted_scope(command, given) {
        return ChosenOrError::Choice(ScopeChoice::Scope { scope });
    }
    let accepted: Vec<&str> = scope_rule(command)
        .accepts
        .iter()
        .map(|scope| scope.as_str())
        .collect();
    ChosenOrError::Error(usage_error(&format!(
        "Invalid scope: {given}. Use one of: {}",
        accepted.join(", ")
    )))
}

/// The scope an omitted `--scope` resolves to: "the most specific scope it's installed at ... checking
/// local, project, user, then managed" (`update`); `enable` and `disable` check "local, project,
/// user" and use "the first scope that mentions the plugin". `None` when none does.
#[must_use]
pub fn detect_scope(command: ScopedCommand, declared_at: &[CommandScope]) -> Option<CommandScope> {
    scope_rule(command)
        .detects
        .iter()
        .copied()
        .find(|scope| declared_at.contains(scope))
}

/// Where `enable` or `disable` writes for a `--scope` given.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ScopeWrite {
    /// The scope that declares it.
    Write {
        /// The scope.
        scope: CommandScope,
    },
    /// "Claude Code writes an override at the scope you passed."
    Override {
        /// The scope.
        scope: CommandScope,
    },
    /// Any other scope.
    Refused {
        /// Why.
        message: String,
    },
    /// No scope declares it.
    NotInstalled,
}

/// `SCOPE_RANK[scope]` of a command scope, the settings scope of its name.
fn scope_rank(scope: CommandScope) -> u8 {
    rank_of(match scope {
        InstallScope::User => PolicyScope::User,
        InstallScope::Project => PolicyScope::Project,
        InstallScope::Local => PolicyScope::Local,
        InstallScope::Managed => PolicyScope::Managed,
    })
}

/// `--scope` given to `enable` or `disable` for a plugin (plugins/cli-reference.md, "plugin enable"):
/// where the plugin is declared the scope is written; "a scope that takes precedence over the
/// declaring one" writes an override; "any other scope" fails with `Plugin "<name>" is installed at
/// <scope> scope, not <given>. Use --scope <scope> or omit --scope to auto-detect.`, naming the scope
/// auto-detect would pick. A plugin no scope declares is the caller's `not-installed`.
#[must_use]
pub fn write_scope(
    plugin: &str,
    requested: CommandScope,
    declared_at: &[CommandScope],
) -> ScopeWrite {
    if declared_at.contains(&requested) {
        return ScopeWrite::Write { scope: requested };
    }
    let strongest = declared_at.iter().map(|scope| scope_rank(*scope)).max();
    if strongest.is_some_and(|strongest| scope_rank(requested) > strongest) {
        return ScopeWrite::Override { scope: requested };
    }
    let Some(found) =
        detect_scope(ScopedCommand::Enable, declared_at).or_else(|| declared_at.first().copied())
    else {
        return ScopeWrite::NotInstalled;
    };
    let (found, requested) = (found.as_str(), requested.as_str());
    ScopeWrite::Refused {
        message: format!(
            "Plugin \"{plugin}\" is installed at {found} scope, not {requested}. Use --scope {found} or omit --scope to auto-detect."
        ),
    }
}

crate::registry::exports! {
    "cli/scope" {
        "CONFIG_SCOPES" => ok || CONFIG_SCOPES;
        "SCOPED_COMMANDS" => ok || SCOPED_COMMANDS;
        "scopeFlagSpelling" => ok |command: ScopedCommand| scope_flag_spelling(command);
        "chooseScope" => ok |command: ScopedCommand, given: Option<String>| choose_scope(command, given.as_deref());
        "detectScope" => ok |command: ScopedCommand, declared_at: Vec<CommandScope>| detect_scope(command, &declared_at);
        "writeScope" => ok |plugin: String, requested: CommandScope, declared_at: Vec<CommandScope>| write_scope(&plugin, requested, &declared_at);
    }
}
