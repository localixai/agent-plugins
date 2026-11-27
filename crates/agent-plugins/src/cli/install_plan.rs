//! Port of `src/cli/install-plan.ts`, switched.
//!
//! The second half of `claude plugin install <plugin>` (plugins/cli-reference.md, "plugin install",
//! "Accept a displayed install command"): the decision ladder once the host has added the
//! `--marketplace` source and refreshed. In order: the plugin is looked up (`Plugin "<name>" not found
//! in any configured marketplace` for a bare name nobody lists); one already installed at that scope
//! ends with `Plugin "<id>" is already installed (scope: <scope>)` and exit `0`, still planning any
//! missing dependency ("re-running `claude plugin install` on the dependent"); the `--config` values
//! are checked; the dependency closure is planned; a command-source or `headersHelper` entry prints its
//! command and needs `-y` or the digest (`decideAcceptance`). The refusals the docs do not quote are in
//! spec/judgments.md.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::command_acceptance::{
    AcceptanceFlags, AcceptanceRequest, CommandAcceptance, DisplayedCommand, ShownCommand,
    decide_acceptance,
};
use super::configure_plan::SavedValues;
use super::install_config::{
    ConfigOutcome, InstallOptions, apply_config, not_applied_lines, not_yet_set_line,
};
use super::install_lookup::{before_lines, displayed_of, locate, marketplace_refusal};
use super::install_refusals::{closure_refusal, config_refusal};
use super::install_start::PendingInstall;
use super::records_refusal::records_refusal;
use super::result::{CommandOutput, Extras, ResultFields, command_failed, command_succeeded};
use crate::format::marketplace::marketplace_names::RegisteredMarketplace;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::marketplace::dependency_graph::{CatalogPlugin, GraphFacts};
use crate::marketplace::dependency_install_plan::{InstallRequest, InstallStep, install_closure};
use crate::marketplace::install_messages::RefusingCommand;
use crate::marketplace::install_scopes::InstallScope;
use crate::ports::marketplace::Sha256;
use crate::registry::Port;

/// The marketplace `--marketplace` added or reused. `registered` is the fetch's fact for the
/// download-folder rule: "A new one is added under the same organization policy checks as `claude
/// plugin marketplace add`", so a name that command refuses refuses the install.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedMarketplace {
    /// Its name.
    pub name: String,
    /// It was added before.
    pub already_known: bool,
    /// The marketplaces registered.
    pub registered: Vec<RegisteredMarketplace>,
    /// When `alreadyKnown`: the source it was added from; another source fails (§17.20.2).
    #[serde(default)]
    pub existing_source: Option<JsValue>,
}

/// What the host found after its `--marketplace` add and refresh.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallFound {
    /// The installed plugins and the marketplace catalogs, after the refresh.
    pub graph: GraphFacts,
    /// The catalog text the run read, after its own refresh: it goes into the digest `--accept-command` names.
    pub catalog: String,
    /// The plugin's `userConfig` and its bundled servers'.
    pub options: InstallOptions,
    /// The marketplace `--marketplace` added or reused.
    #[serde(default)]
    pub added_marketplace: Option<AddedMarketplace>,
    /// The refresh the plan asked for failed: "the install proceeds from the cached catalog".
    #[serde(default)]
    pub refresh_failed: Option<bool>,
    /// The installed copy's directory exists; `false` for a record whose directory is gone ("not cached"), which a reinstall restores.
    #[serde(default)]
    pub installed_copy_present: Option<bool>,
    /// The answer to `Run this command now? [y/N]`, once the host has asked.
    #[serde(default)]
    pub answer: Option<bool>,
    /// The text `readInstalledPlugins` reported for a record of `installed_plugins.json` this version
    /// cannot read, or a format it does not know; the command then refuses (§16.6.2 item 1).
    #[serde(default)]
    pub unreadable_records: Option<String>,
}

/// What the host carries out for a successful install.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InstallEffects {
    /// The plugin.
    pub id: String,
    /// Its scope.
    pub scope: InstallScope,
    /// Dependencies first; the plugin itself is installed after them unless it is already installed.
    pub steps: Vec<InstallStep>,
    /// Install the plugin (not for one that was already installed).
    pub install: bool,
    /// Write `enabledPlugins[<id>]: true` at the scope: not for one already installed, nor for one
    /// whose `defaultEnabled` is `false`, which "is installed but stays off until you turn it on"
    /// (install.md, "Choose an install scope"). Every step is enabled whatever its own default.
    pub enable: bool,
    /// The plugin's own values.
    pub config: SavedValues,
    /// Values for each bundled server, by server name.
    pub servers: IndexMap<String, JsObject>,
}

/// What the host does next.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum InstallRun {
    /// The host prints `before`, asks `prompt` and calls `planInstall` again with the answer.
    Ask {
        /// What prints first.
        before: Vec<String>,
        /// The question.
        prompt: String,
    },
    /// The plan `needs` catalog plugins fetched (`InstallPlan.needs`): the host fills them in `graph` and calls `planInstall` again.
    Needs {
        /// What prints first.
        before: Vec<String>,
        /// The plugins to fetch.
        needs: Vec<String>,
    },
    /// The command ends.
    Ended {
        /// What prints first.
        before: Vec<String>,
        /// What it prints.
        output: CommandOutput,
        /// What the host carries out.
        #[serde(skip_serializing_if = "Option::is_none")]
        effects: Option<Box<InstallEffects>>,
    },
}

const fn ended(output: CommandOutput, before: Vec<String>) -> InstallRun {
    InstallRun::Ended {
        before,
        output,
        effects: None,
    }
}

fn fields_of(id: &str, scope: InstallScope) -> ResultFields {
    [
        ("pluginId".to_owned(), JsValue::from(id)),
        ("scope".to_owned(), JsValue::from(scope.as_str())),
    ]
    .into_iter()
    .collect()
}

fn shown_fields(shown: &ShownCommand, id: &str, scope: InstallScope) -> ResultFields {
    let mut fields = fields_of(id, scope);
    fields.insert(
        "shownCommand".to_owned(),
        to_js_value(shown).unwrap_or_default(),
    );
    fields
}

/// What `notRun` reads of where the command stopped.
struct NotRun<'a> {
    id: &'a str,
    scope: InstallScope,
    before: &'a [String],
    command: &'a str,
}

/// A run that printed the command and did not run it.
fn not_run(line: &str, shown: &ShownCommand, context: &NotRun<'_>) -> InstallRun {
    let mut before = context.before.to_vec();
    before.push(context.command.to_owned());
    ended(
        command_failed(
            "install",
            line,
            Extras {
                failure_code: Some("command_not_accepted".to_owned()),
                fields: Some(shown_fields(shown, context.id, context.scope)),
            },
        ),
        before,
    )
}

fn already_installed(
    id: &str,
    pending: &PendingInstall,
    found: &InstallFound,
    before: Vec<String>,
) -> Result<InstallRun, String> {
    let closure = install_closure(&request(id, pending.scope, "reinstall"), &found.graph)?;
    if !closure.needs.is_empty() {
        return Ok(InstallRun::Needs {
            before,
            needs: closure.needs,
        });
    }
    let message = format!(
        "Plugin \"{id}\" is already installed (scope: {})",
        pending.scope.as_str()
    );
    let output = command_succeeded(
        "install",
        &message,
        Extras {
            failure_code: None,
            fields: Some(fields_of(id, pending.scope)),
        },
    );
    if !closure.ok || closure.steps.is_empty() {
        return Ok(ended(output, before));
    }
    Ok(InstallRun::Ended {
        before,
        output,
        effects: Some(Box::new(InstallEffects {
            id: id.to_owned(),
            scope: pending.scope,
            steps: closure.steps,
            install: false,
            enable: false,
            config: SavedValues::default(),
            servers: IndexMap::new(),
        })),
    })
}

fn request(root: &str, scope: InstallScope, operation: &str) -> InstallRequest {
    InstallRequest {
        root: root.to_owned(),
        scope,
        operation: operation.to_owned(),
        host_version: None,
    }
}

fn is_installed_at(id: &str, scope: InstallScope, graph: &GraphFacts) -> bool {
    graph
        .installed
        .iter()
        .any(|record| record.id == id && record.scope == scope)
}

/// What the success lines read.
struct Succeeded<'a> {
    id: &'a str,
    scope: InstallScope,
    steps: &'a [InstallStep],
    config: &'a ConfigOutcome,
    options: &'a InstallOptions,
    left_out: Vec<String>,
}

/// "The success message lists them": ` (+ N dependencies: <ids>)` (wording: spec/judgments.md).
fn dependency_list(steps: &[InstallStep]) -> String {
    if steps.is_empty() {
        return String::new();
    }
    let ids: Vec<&str> = steps.iter().map(|step| step.id.as_str()).collect();
    format!(" (+ {} dependencies: {})", steps.len(), ids.join(", "))
}

/// The success lines: the install, what `--config` did not apply, the options still unset, dependencies left to the user.
fn success_lines(done: Succeeded<'_>) -> String {
    let unset = not_yet_set_line(done.id, done.config);
    std::iter::once(format!(
        "Successfully installed plugin: {} (scope: {}){}",
        done.id,
        done.scope.as_str(),
        dependency_list(done.steps)
    ))
    .chain(not_applied_lines(done.config, done.options))
    .chain(unset)
    .chain(done.left_out)
    .collect::<Vec<_>>()
    .join("\n")
}

async fn acceptance_of(
    shown: &DisplayedCommand,
    pending: &PendingInstall,
    found: &InstallFound,
    sha256: &dyn Sha256,
) -> CommandAcceptance {
    decide_acceptance(
        &AcceptanceRequest {
            shown: shown.clone(),
            catalog: found.catalog.clone(),
            flags: AcceptanceFlags {
                yes: pending.yes.then_some(true),
                accept_command: pending.accept_command.clone(),
            },
            session: pending.session,
            tty: pending.tty,
            answer: found.answer,
        },
        sha256,
    )
    .await
}

/// The run that ends the install at its command: the question, or the refusal. `None` when the command runs or the entry has none.
async fn stop_at_command(
    (id, entry, before): (&str, &CatalogPlugin, &[String]),
    pending: &PendingInstall,
    found: &InstallFound,
    sha256: &dyn Sha256,
) -> Option<InstallRun> {
    let shown = displayed_of(id, entry)?;
    match acceptance_of(&shown, pending, found, sha256).await {
        CommandAcceptance::Run { .. } => None,
        CommandAcceptance::Ask { prompt, .. } => {
            let mut lines = before.to_vec();
            lines.push(shown.command.clone());
            Some(InstallRun::Ask {
                before: lines,
                prompt,
            })
        }
        CommandAcceptance::Line {
            line,
            shown_command,
            ..
        } => Some(not_run(
            &line,
            &shown_command,
            &NotRun {
                id,
                scope: pending.scope,
                before,
                command: &shown.command,
            },
        )),
    }
}

/// What `claude plugin install` prints and does, once the host added the marketplace and refreshed.
///
/// # Errors
///
/// What the dependency closure throws for a range it cannot read.
pub async fn plan_install(
    pending: &PendingInstall,
    found: &InstallFound,
    sha256: &dyn Sha256,
) -> Result<InstallRun, String> {
    let before = before_lines(found);
    let records = records_refusal(
        RefusingCommand::Install,
        &pending.plugin,
        found.unreadable_records.as_deref(),
    );
    if let Some(records) = records {
        return Ok(ended(records, before));
    }
    if let Some(refused) = marketplace_refusal(pending, found) {
        return Ok(ended(refused, Vec::new()));
    }
    let (id, entry) = match locate(pending, found) {
        Ok(located) => located,
        Err(end) => return Ok(ended(*end, before)),
    };
    let scope = pending.scope;
    if is_installed_at(&id, scope, &found.graph) && found.installed_copy_present != Some(false) {
        return already_installed(&id, pending, found, before);
    }
    let config = apply_config(&pending.given, &found.options);
    if !config.refused.is_empty() {
        return Ok(ended(config_refusal(&config), before));
    }
    let closure = install_closure(&request(&id, scope, "install"), &found.graph)?;
    if !closure.needs.is_empty() {
        return Ok(InstallRun::Needs {
            before,
            needs: closure.needs,
        });
    }
    if !closure.ok {
        let messages: Vec<String> = closure
            .issues
            .into_iter()
            .map(|issue| issue.message)
            .collect();
        return Ok(ended(closure_refusal(&id, &messages), before));
    }
    if let Some(stopped) = stop_at_command((&id, entry, &before), pending, found, sha256).await {
        return Ok(stopped);
    }
    let left_out = closure
        .left_out
        .iter()
        .map(|one| one.message.clone())
        .collect();
    let lines = success_lines(Succeeded {
        id: &id,
        scope,
        steps: &closure.steps,
        config: &config,
        options: &found.options,
        left_out,
    });
    Ok(InstallRun::Ended {
        before,
        output: command_succeeded(
            "install",
            &lines,
            Extras {
                failure_code: None,
                fields: Some(fields_of(&id, scope)),
            },
        ),
        effects: Some(Box::new(InstallEffects {
            enable: entry.default_enabled != Some(false),
            id,
            scope,
            steps: closure.steps,
            install: true,
            config: config.plugin,
            servers: config.servers,
        })),
    })
}

crate::registry::exports! {
    "cli/install-plan" {
        "planInstall" => async fallible |pending: PendingInstall, found: InstallFound, sha256: Port|
            plan_install(&pending, &found, &sha256).await;
    }
}
