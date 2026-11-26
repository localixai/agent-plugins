//! Port of `src/cli/result.ts`, switched.
//!
//! What a `claude plugin` command reports (plugins/cli-reference.md, "claude plugin commands",
//! "JSON result format", "JSON result for marketplace commands"): the human line, the exit code
//! and, with `--json`, one object on the last line of stdout. The kit has no CLI of its own, so a
//! command is a plan whose outcome is one of these values; a host prints `renderCommand`'s lines
//! and exits with its code. The streams a human line goes to, and the exit code of a refusal that
//! prints no result line, are in spec/judgments.md.

use serde::{Deserialize, Serialize};

use crate::foundation::json_value::to_json_value;
use crate::js::{JsObject, JsValue, in_property_order, json_stringify};

/// `ok` or `failed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandOutcome {
    /// `ok`.
    Ok,
    /// `failed`.
    Failed,
}

/// Every outcome.
pub const COMMAND_OUTCOMES: [CommandOutcome; 2] = [CommandOutcome::Ok, CommandOutcome::Failed];

/// `0` success, `1` failure, `2` an unexpected error (`validate` only).
pub type ExitCode = u8;

/// Fields beyond the three that are always present: `marketplace`, `pluginId`, `scope`…
pub type ResultFields = JsObject;

/// One command's result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandResult {
    /// The subcommand that ran, such as `install` or `marketplace-add`.
    pub command: String,
    /// Whether it succeeded.
    pub outcome: CommandOutcome,
    /// The human-readable description of the result, the line printed without `--json`.
    pub message: String,
    /// Why a command failed, such as `invalid_source`; present only when it applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
    /// The other fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<ResultFields>,
}

/// What a command ends with. Only `result` can print the JSON line: a usage error ("an invalid
/// `--scope`"), a name that "can print no result line" and an unexpected error give a reason on
/// stderr alone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CommandOutput {
    /// `result`.
    Result {
        /// The result.
        result: PluginCommandResult,
    },
    /// `usage-error`.
    UsageError {
        /// Why.
        reason: String,
    },
    /// `no-result-line`.
    NoResultLine {
        /// Why.
        reason: String,
    },
    /// `unexpected-error`.
    UnexpectedError {
        /// Why.
        reason: String,
    },
}

/// What a host prints and exits with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rendered {
    /// The lines of stdout.
    pub stdout: Vec<String>,
    /// The lines of stderr.
    pub stderr: Vec<String>,
    /// The exit code.
    pub exit_code: ExitCode,
}

/// How a command is rendered.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderOptions {
    /// `--json`: the last line of stdout is the result object instead of the human line.
    pub json: bool,
    /// What the command printed before its result (a marketplace-declared command, for one).
    #[serde(default)]
    pub before: Option<Vec<String>>,
}

/// A result's optional members.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extras {
    /// `failureCode`.
    #[serde(default)]
    pub failure_code: Option<String>,
    /// `fields`.
    #[serde(default)]
    pub fields: Option<ResultFields>,
}

fn with_extras(base: PluginCommandResult, extras: Extras) -> PluginCommandResult {
    PluginCommandResult {
        failure_code: extras.failure_code,
        fields: extras.fields,
        ..base
    }
}

const fn resulted(result: PluginCommandResult) -> CommandOutput {
    CommandOutput::Result { result }
}

/// A command that succeeded.
#[must_use]
pub fn command_succeeded(command: &str, message: &str, extras: Extras) -> CommandOutput {
    let base = PluginCommandResult {
        command: command.to_owned(),
        outcome: CommandOutcome::Ok,
        message: message.to_owned(),
        failure_code: None,
        fields: None,
    };
    resulted(with_extras(base, extras))
}

/// A command that failed.
#[must_use]
pub fn command_failed(command: &str, message: &str, extras: Extras) -> CommandOutput {
    let base = PluginCommandResult {
        command: command.to_owned(),
        outcome: CommandOutcome::Failed,
        message: message.to_owned(),
        failure_code: None,
        fields: None,
    };
    resulted(with_extras(base, extras))
}

/// A usage error ("an invalid `--scope`").
#[must_use]
pub fn usage_error(reason: &str) -> CommandOutput {
    CommandOutput::UsageError {
        reason: reason.to_owned(),
    }
}

/// A name that "can print no result line".
#[must_use]
pub fn no_result_line(reason: &str) -> CommandOutput {
    CommandOutput::NoResultLine {
        reason: reason.to_owned(),
    }
}

/// An unexpected error.
#[must_use]
pub fn unexpected_error(reason: &str) -> CommandOutput {
    CommandOutput::UnexpectedError {
        reason: reason.to_owned(),
    }
}

/// The `command` values of the marketplace commands ("`marketplace-add`, `marketplace-remove`,
/// or `marketplace-update`").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarketplaceCommand {
    /// `marketplace-add`.
    MarketplaceAdd,
    /// `marketplace-remove`.
    MarketplaceRemove,
    /// `marketplace-update`.
    MarketplaceUpdate,
}

/// Every marketplace command.
pub const MARKETPLACE_COMMANDS: [MarketplaceCommand; 3] = [
    MarketplaceCommand::MarketplaceAdd,
    MarketplaceCommand::MarketplaceRemove,
    MarketplaceCommand::MarketplaceUpdate,
];

impl MarketplaceCommand {
    /// The command's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MarketplaceAdd => "marketplace-add",
            Self::MarketplaceRemove => "marketplace-remove",
            Self::MarketplaceUpdate => "marketplace-update",
        }
    }
}

fn marketplace_fields(marketplace: Option<&str>) -> Extras {
    Extras {
        failure_code: None,
        fields: marketplace.map(|marketplace| {
            JsObject::from([(
                "marketplace".to_owned(),
                JsValue::String(marketplace.to_owned()),
            )])
        }),
    }
}

/// A marketplace command that succeeded.
#[must_use]
pub fn marketplace_success(
    command: MarketplaceCommand,
    marketplace: Option<&str>,
    message: &str,
) -> CommandOutput {
    command_succeeded(command.name(), message, marketplace_fields(marketplace))
}

/// A marketplace command that failed.
#[must_use]
pub fn marketplace_failure(
    command: MarketplaceCommand,
    marketplace: Option<&str>,
    message: &str,
    failure_code: &str,
) -> CommandOutput {
    let extras = Extras {
        failure_code: Some(failure_code.to_owned()),
        ..marketplace_fields(marketplace)
    };
    command_failed(command.name(), message, extras)
}

/// The three keys "always present": a key of `fields` with one of these names is dropped
/// (spec/judgments.md).
const ALWAYS_PRESENT: [&str; 3] = ["command", "outcome", "message"];

fn other_fields(fields: Option<&ResultFields>) -> impl Iterator<Item = (String, JsValue)> + '_ {
    fields
        .into_iter()
        .flatten()
        .filter(|(key, _)| !ALWAYS_PRESENT.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
}

/// The object of the last line: `command`, `outcome`, the other fields, `failureCode`, `message`.
fn result_object(found: &PluginCommandResult) -> JsObject {
    let outcome = match found.outcome {
        CommandOutcome::Ok => "ok",
        CommandOutcome::Failed => "failed",
    };
    let mut object = JsObject::new();
    object.insert("command".to_owned(), JsValue::String(found.command.clone()));
    object.insert("outcome".to_owned(), JsValue::String(outcome.to_owned()));
    // A spread sets a key already written in place: a field named `failureCode` stays where the
    // fields put it, and the result's own `failureCode` overwrites its value there.
    object.extend(other_fields(found.fields.as_ref()));
    if let Some(code) = &found.failure_code {
        object.insert("failureCode".to_owned(), JsValue::String(code.clone()));
    }
    object.insert("message".to_owned(), JsValue::String(found.message.clone()));
    in_property_order(object)
}

const fn exit(outcome: CommandOutcome) -> ExitCode {
    match outcome {
        CommandOutcome::Ok => 0,
        CommandOutcome::Failed => 1,
    }
}

/// A result's human line goes to stdout on success and stderr on failure; with `--json` both are
/// the last stdout line.
fn render_result(found: &PluginCommandResult, options: &RenderOptions) -> Rendered {
    let mut stdout = options.before.clone().unwrap_or_default();
    let exit_code = exit(found.outcome);
    if options.json {
        let line = to_json_value(&JsValue::Object(result_object(found)));
        // An object always has a text: `JSON.stringify` never answers `undefined` here.
        stdout.push(line.as_ref().and_then(json_stringify).unwrap_or_default());
        return Rendered {
            stdout,
            stderr: Vec::new(),
            exit_code,
        };
    }
    // The stream a result's human line goes to (`HUMAN_STREAM`).
    let on_stderr = found.outcome == CommandOutcome::Failed;
    if !on_stderr {
        stdout.push(found.message.clone());
    }
    Rendered {
        stdout,
        stderr: if on_stderr {
            vec![found.message.clone()]
        } else {
            Vec::new()
        },
        exit_code,
    }
}

fn reason_only(reason: &str, exit_code: ExitCode) -> Rendered {
    Rendered {
        stdout: Vec::new(),
        stderr: vec![reason.to_owned()],
        exit_code,
    }
}

/// What a host prints and exits with for `output` (`RENDERERS`).
#[must_use]
pub fn render_command(output: &CommandOutput, options: &RenderOptions) -> Rendered {
    match output {
        CommandOutput::Result { result } => render_result(result, options),
        CommandOutput::UsageError { reason } | CommandOutput::NoResultLine { reason } => {
            reason_only(reason, 1)
        }
        CommandOutput::UnexpectedError { reason } => reason_only(reason, 2),
    }
}

crate::registry::exports! {
    "cli/result" {
        "COMMAND_OUTCOMES" => ok || COMMAND_OUTCOMES;
        "commandSucceeded" => ok |command: String, message: String, extras: Option<Extras>| command_succeeded(&command, &message, extras.unwrap_or_default());
        "commandFailed" => ok |command: String, message: String, extras: Option<Extras>| command_failed(&command, &message, extras.unwrap_or_default());
        "usageError" => ok |reason: String| usage_error(&reason);
        "noResultLine" => ok |reason: String| no_result_line(&reason);
        "unexpectedError" => ok |reason: String| unexpected_error(&reason);
        "MARKETPLACE_COMMANDS" => ok || MARKETPLACE_COMMANDS;
        "marketplaceSuccess" => ok |command: MarketplaceCommand, marketplace: Option<String>, message: String| marketplace_success(command, marketplace.as_deref(), &message);
        "marketplaceFailure" => ok |command: MarketplaceCommand, marketplace: Option<String>, message: String, failure_code: String| marketplace_failure(command, marketplace.as_deref(), &message, &failure_code);
        "renderCommand" => ok |output: CommandOutput, options: RenderOptions| render_command(&output, &options);
    }
}
