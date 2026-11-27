//! Port of `src/cli/update-plan.ts`, switched.
//!
//! `claude plugin update <plugin>` as a plan (plugins/cli-reference.md, "plugin update"): "Claude Code
//! prints `Checking for updates for plugin "<plugin>"…`, then the result. When nothing is newer, it
//! prints `<name> is already at the latest version (<version>).` and exits `0`, unless it retries the
//! plugin's dependency install and that install fails". Without `--scope` the plugin is updated "at the
//! most specific scope it's installed at ... local, project, user, then managed"; bare names and the
//! qualified commands are `locate`'s; "When the plugin is already at its latest version, the command
//! can also retry an unfinished dependency install in its cached copy" (plugins/troubleshooting.md,
//! "The packages it lists are not installed": `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` skips the
//! retry); `-y` and `--accept-command` accept "a changed install command from a command-source
//! plugin" (`commandSourceRuns`, `decideAcceptance`). What the docs leave open is in spec/judgments.md.
//!
//! The environment and the digest cross as ports beside the facts; the install the facts list is the
//! caller's value, read where the TypeScript reads it.

use serde::{Deserialize, Serialize};

use super::command_acceptance::{
    AcceptanceFlags, AcceptanceRequest, CommandAcceptance, DisplayedCommand, decide_acceptance,
    exclusive_accept_flags,
};
use super::plugin_ref::name_of;
use super::records_refusal::records_refusal;
use super::result::{CommandOutput, Extras, ResultFields, command_failed, command_succeeded};
use super::scope::{ChosenOrError, ScopeChoice, ScopedCommand, choose_scope};
use super::update_locate::{
    LocateArguments, LocateFacts, Located, command_to_accept, failure_line, locate,
};
use crate::js::JsValue;
use crate::marketplace::install_messages::RefusingCommand;
use crate::marketplace::install_scopes::InstallScope;
use crate::marketplace::update_detection::{
    UpdateCheck, UpdateSubject, check_for_update, up_to_date_message,
};
use crate::marketplace::version_resolution::UNKNOWN_VERSION;
use crate::ports::environment::Environment;
use crate::ports::marketplace::Sha256;
use crate::updates::env_flags::{DISABLE_NONESSENTIAL_TRAFFIC, is_set};

/// The command line.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateArguments {
    /// `name` or `name@marketplace`.
    pub plugin: String,
    /// `--scope`.
    #[serde(default)]
    pub scope: Option<String>,
    /// `-y`.
    #[serde(default)]
    pub yes: Option<bool>,
    /// `--accept-command`.
    #[serde(default)]
    pub accept_command: Option<String>,
}

/// What the host read, but the environment (its own port).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFacts {
    /// The plugins installed for the current project, at every scope.
    pub installs: Vec<JsValue>,
    /// The catalog's text, for the digest.
    pub catalog: String,
    /// The run is an interactive session.
    pub session: bool,
    /// Both stdin and stdout are a terminal.
    pub tty: bool,
    /// The answer to the prompt, on the second call.
    #[serde(default)]
    pub answer: Option<bool>,
    /// The install records could not be read: why.
    #[serde(default)]
    pub unreadable_records: Option<String>,
}

/// What the host carries out for an update that found a newer version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpdateEffects {
    /// The plugin.
    pub id: String,
    /// Its scope.
    pub scope: InstallScope,
    /// The version it is updated to.
    pub to: String,
    /// The version it had.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

/// A retry of the dependency install in the cached copy: the host runs it and reports with `concludeRetry`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetryRequest {
    /// The plugin.
    pub id: String,
    /// Its scope.
    pub scope: InstallScope,
    /// The plugin as typed.
    pub argument: String,
    /// The latest-version line.
    pub latest: String,
    /// The result's fields.
    pub fields: ResultFields,
}

/// What the host does next.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum UpdateRun {
    /// The host prints `before`, asks `prompt` and calls again with the answer.
    Prompt {
        /// The lines first.
        before: Vec<String>,
        /// The question.
        prompt: String,
    },
    /// The host retries the dependency install.
    Retry {
        /// The lines first.
        before: Vec<String>,
        /// The retry.
        retry: RetryRequest,
    },
    /// The result.
    Ended {
        /// The lines first.
        before: Vec<String>,
        /// What it prints.
        output: CommandOutput,
        /// The update to carry out.
        #[serde(skip_serializing_if = "Option::is_none")]
        update: Option<UpdateEffects>,
    },
}

/// How the host's retry went.
#[derive(Clone, Debug, Deserialize)]
pub struct RetryOutcome {
    /// It succeeded.
    pub ok: bool,
    /// Why it failed.
    #[serde(default)]
    pub reason: Option<String>,
}

const fn ended(output: CommandOutput, before: Vec<String>) -> UpdateRun {
    UpdateRun::Ended {
        before,
        output,
        update: None,
    }
}

fn checking(argument: &str) -> Vec<String> {
    vec![format!("Checking for updates for plugin \"{argument}\"…")]
}

fn succeeded(message: &str, fields: ResultFields) -> CommandOutput {
    let extras = Extras {
        failure_code: None,
        fields: Some(fields),
    };
    command_succeeded("update", message, extras)
}

fn failed(message: &str, code: &str, fields: ResultFields) -> CommandOutput {
    let extras = Extras {
        failure_code: Some(code.to_owned()),
        fields: Some(fields),
    };
    command_failed("update", message, extras)
}

fn fields_of(id: &str, scope: InstallScope) -> ResultFields {
    [("pluginId", id.into()), ("scope", scope.as_str().into())]
        .into_iter()
        .map(|(key, value): (&str, JsValue)| (key.to_owned(), value))
        .collect()
}

/// The members of an install the plan reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Install {
    id: String,
    scope: InstallScope,
    #[serde(default)]
    installed_version: Option<String>,
    #[serde(default)]
    dependency_install: Option<String>,
    subject: UpdateSubject,
}

/// The latest-version result, with the dependency-install retry when it is due.
async fn at_latest(install: &Install, argument: &str, env: &dyn Environment) -> UpdateRun {
    let version = install
        .installed_version
        .as_deref()
        .unwrap_or(UNKNOWN_VERSION);
    let latest = up_to_date_message(&name_of(&install.id), version);
    let fields = fields_of(&install.id, install.scope);
    let unfinished = install.dependency_install.as_deref() == Some("unfinished");
    if !unfinished || is_set(env, DISABLE_NONESSENTIAL_TRAFFIC).await {
        return ended(succeeded(&latest, fields), checking(argument));
    }
    let retry = RetryRequest {
        id: install.id.clone(),
        scope: install.scope,
        argument: argument.to_owned(),
        latest,
        fields,
    };
    UpdateRun::Retry {
        before: checking(argument),
        retry,
    }
}

/// The result once the host has retried the dependency install: the failure fails the command, the success leaves the latest-version line.
#[must_use]
pub fn conclude_retry(request: RetryRequest, outcome: &RetryOutcome) -> UpdateRun {
    let before = checking(&request.argument);
    if outcome.ok {
        return ended(succeeded(&request.latest, request.fields), before);
    }
    let reason = outcome.reason.as_deref().unwrap_or_default();
    let line = failure_line(&request.argument, reason);
    ended(
        failed(&line, "dependency_install_failed", request.fields),
        before,
    )
}

/// The run that stops at the command: the question or the refusal; `None` when it runs.
fn stopped_at(
    acceptance: CommandAcceptance,
    install: &Install,
    shown: &DisplayedCommand,
    argument: &str,
) -> Option<UpdateRun> {
    let mut before = checking(argument);
    before.push(shown.command.clone());
    match acceptance {
        CommandAcceptance::Run { .. } => None,
        CommandAcceptance::Ask { prompt, .. } => Some(UpdateRun::Prompt { before, prompt }),
        CommandAcceptance::Line {
            line,
            shown_command,
            ..
        } => {
            let mut fields = fields_of(&install.id, install.scope);
            let shown = crate::js::value::to_js_value(&shown_command).unwrap_or(JsValue::Undefined);
            fields.insert("shownCommand".to_owned(), shown);
            Some(ended(failed(&line, "command_not_accepted", fields), before))
        }
    }
}

/// The ports the plan asks.
struct Hosts<'a> {
    env: &'a dyn Environment,
    sha256: &'a dyn Sha256,
}

async fn update_to(
    raw: &JsValue,
    install: &Install,
    to: String,
    args: &UpdateArguments,
    facts: &UpdateFacts,
    hosts: &Hosts<'_>,
) -> UpdateRun {
    if let Some(shown) = command_to_accept(raw, hosts.env).await {
        let request = AcceptanceRequest {
            shown: shown.clone(),
            catalog: facts.catalog.clone(),
            flags: AcceptanceFlags {
                yes: args.yes.filter(|yes| *yes),
                accept_command: args.accept_command.clone(),
            },
            session: facts.session,
            tty: facts.tty,
            answer: facts.answer,
        };
        let acceptance = decide_acceptance(&request, hosts.sha256).await;
        if let Some(stopped) = stopped_at(acceptance, install, &shown, &args.plugin) {
            return stopped;
        }
    }
    let message = format!(
        "Successfully updated plugin: {} to version {to} (scope: {})",
        name_of(&install.id),
        install.scope.as_str()
    );
    UpdateRun::Ended {
        before: checking(&args.plugin),
        output: succeeded(&message, fields_of(&install.id, install.scope)),
        update: Some(UpdateEffects {
            id: install.id.clone(),
            scope: install.scope,
            to,
            from: install.installed_version.clone(),
        }),
    }
}

/// What `claude plugin update` prints and does.
///
/// # Errors
///
/// The install located is not an install's shape.
pub async fn plan_plugin_update(
    args: &UpdateArguments,
    facts: &UpdateFacts,
    env: &dyn Environment,
    sha256: &dyn Sha256,
) -> Result<UpdateRun, String> {
    let chosen = match choose_scope(ScopedCommand::Update, args.scope.as_deref()) {
        ChosenOrError::Error(output) => return Ok(ended(output, Vec::new())),
        ChosenOrError::Choice(ScopeChoice::Scope { scope }) => Some(scope),
        ChosenOrError::Choice(ScopeChoice::AutoDetect { .. } | ScopeChoice::AllScopes { .. }) => {
            None
        }
    };
    let flags = AcceptanceFlags {
        yes: args.yes,
        accept_command: args.accept_command.clone(),
    };
    if let Some(exclusive) = exclusive_accept_flags(&flags) {
        return Ok(ended(exclusive, Vec::new()));
    }
    if let Some(records) = records_refusal(
        RefusingCommand::Update,
        &args.plugin,
        facts.unreadable_records.as_deref(),
    ) {
        return Ok(ended(records, Vec::new()));
    }
    let located = locate(
        &LocateArguments {
            plugin: args.plugin.clone(),
        },
        &LocateFacts {
            installs: facts.installs.clone(),
        },
        chosen,
    );
    let raw = match located {
        Located::End { end } => return Ok(ended(end, Vec::new())),
        Located::Install { install } => install,
    };
    let install = Install::deserialize(raw.clone()).map_err(|problem| problem.to_string())?;
    let verdict = check_for_update(
        &install.subject,
        install.installed_version.as_deref(),
        sha256,
    )
    .await;
    Ok(if verdict.check == UpdateCheck::NewCopy {
        update_to(
            &raw,
            &install,
            verdict.version,
            args,
            facts,
            &Hosts { env, sha256 },
        )
        .await
    } else {
        at_latest(&install, &args.plugin, env).await
    })
}

crate::registry::exports! {
    "cli/update-plan" {
        "concludeRetry" => ok |request: RetryRequest, outcome: RetryOutcome| conclude_retry(request, &outcome);
        "planPluginUpdate" => async fallible |args: UpdateArguments, facts: UpdateFacts, env: crate::registry::Port, sha256: crate::registry::Port|
            plan_plugin_update(&args, &facts, &env, &sha256).await;
    }
}
