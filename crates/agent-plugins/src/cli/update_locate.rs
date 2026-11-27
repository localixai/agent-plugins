//! Ported from `src/cli/update-locate.ts`; only the core uses it now, so it has no wrapper.
//!
//! Which install `claude plugin update` updates and which command it needs accepted (plugins/
//! cli-reference.md, "plugin update", "Which scope the command updates", "Update by bare name").
//!
//! An install (`UpdatableInstall` of `update-facts.ts`, a module of types only) is the caller's
//! value, answered as given; its members are read where the TypeScript reads them.

use serde::{Deserialize, Serialize};

use super::command_acceptance::{CommandOrigin, DisplayedCommand};
use super::plugin_ref::{matching_ids, qualified_commands};
use super::result::{CommandOutput, Extras, command_failed};
use super::scope::{CommandScope, ScopedCommand, detect_scope};
use crate::js::JsValue;
use crate::marketplace::install_scopes::{INSTALL_SCOPES, InstallScope};
use crate::ports::environment::Environment;
use crate::updates::command_source_rerun::{
    CommandDeclaration, CommandPlugin, RerunEvent, command_source_runs,
};

/// `Failed to update plugin "<argument>": <reason>`.
#[must_use]
pub fn failure_line(argument: &str, reason: &str) -> String {
    format!("Failed to update plugin \"{argument}\": {reason}")
}

/// The install, or the output that ends the command.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Located {
    /// The install to update.
    Install {
        /// It.
        install: JsValue,
    },
    /// The end.
    End {
        /// What it prints.
        end: CommandOutput,
    },
}

/// The member of `PluginUpdateArguments` the locating reads.
#[derive(Clone, Debug, Deserialize)]
pub struct LocateArguments {
    /// `name` or `name@marketplace`.
    pub plugin: String,
}

/// The member of `PluginUpdateFacts` the locating reads.
#[derive(Clone, Debug, Deserialize)]
pub struct LocateFacts {
    /// The plugins installed for the current project, at every scope.
    pub installs: Vec<JsValue>,
}

fn id_of(install: &JsValue) -> &str {
    install.get("id").and_then(JsValue::as_str).unwrap_or("")
}

fn scope_of(install: &JsValue) -> Option<InstallScope> {
    let name = install.get("scope").and_then(JsValue::as_str)?;
    INSTALL_SCOPES
        .into_iter()
        .find(|scope| scope.as_str() == name)
}

fn failed(argument: &str, reason: &str, code: &str) -> Located {
    let extras = Extras {
        failure_code: Some(code.to_owned()),
        fields: None,
    };
    Located::End {
        end: command_failed("update", &failure_line(argument, reason), extras),
    }
}

/// The install to update: the plugin the argument names at the scope given or detected.
#[must_use]
pub fn locate(
    args: &LocateArguments,
    facts: &LocateFacts,
    chosen: Option<CommandScope>,
) -> Located {
    let here: Vec<&JsValue> = facts
        .installs
        .iter()
        .filter(|one| chosen.is_none() || scope_of(one) == chosen)
        .collect();
    let ids: Vec<String> = here.iter().map(|one| id_of(one).to_owned()).collect();
    let found = matching_ids(&args.plugin, &ids);
    let Some(id) = found.first() else {
        let place =
            chosen.map_or_else(String::new, |scope| format!(" at scope {}", scope.as_str()));
        let reason = format!("Plugin \"{}\" is not installed{place}", args.plugin);
        return failed(&args.plugin, &reason, "not_installed");
    };
    if found.len() > 1 {
        let commands = qualified_commands("update", &found).join(", ");
        let reason = format!(
            "\"{}\" is installed from more than one marketplace; run one of: {commands}",
            args.plugin
        );
        return failed(&args.plugin, &reason, "ambiguous_plugin");
    }
    let scopes: Vec<InstallScope> = here
        .iter()
        .filter(|one| id_of(one) == id)
        .filter_map(|one| scope_of(one))
        .collect();
    let scope = chosen.or_else(|| detect_scope(ScopedCommand::Update, &scopes));
    match here
        .iter()
        .find(|one| id_of(one) == id && scope.is_some() && scope_of(one) == scope)
    {
        None => {
            let reason = format!("Plugin \"{}\" is not installed", args.plugin);
            failed(&args.plugin, &reason, "not_installed")
        }
        Some(install) => Located::Install {
            install: (*install).clone(),
        },
    }
}

fn declaration(value: Option<&JsValue>) -> Option<CommandDeclaration> {
    value
        .filter(|value| !value.is_undefined())
        .and_then(|value| CommandDeclaration::deserialize(value.clone()).ok())
}

/// What the entry asks Claude Code to run on this update, when the user has to accept it.
pub async fn command_to_accept(
    install: &JsValue,
    env: &dyn Environment,
) -> Option<DisplayedCommand> {
    if let Some(helper) = install
        .get("headersHelper")
        .filter(|helper| !helper.is_undefined())
    {
        return Some(DisplayedCommand {
            command: crate::js::string_of(helper),
            plugin: id_of(install).to_owned(),
            origin: CommandOrigin::HeadersHelper,
        });
    }
    let command = install
        .get("command")
        .filter(|command| !command.is_undefined())?;
    let declared = declaration(command.get("declared"))?;
    let plugin = CommandPlugin {
        id: id_of(install).to_owned(),
        enabled: true,
        installed_version_in_cache: true,
        declared: declared.clone(),
        accepted: declaration(command.get("accepted")),
    };
    let plan = command_source_runs(RerunEvent::Update, &plugin, env).await;
    plan.acceptance
        .is_some_and(|acceptance| acceptance.ask)
        .then_some(DisplayedCommand {
            command: declared.command,
            plugin: plugin.id,
            origin: CommandOrigin::CommandSource,
        })
}
