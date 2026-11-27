//! Port of `src/hooks/handlers/command/run-command.ts`, switched.
//!
//! A `command` hook (hooks.md, "Command hook fields"): the form it runs in, the process it gets,
//! and the run the port reports. What the exit code means is the interpreter's.

use indexmap::IndexMap;
use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize};

use super::r#async::{MODES, ModeFields, mode_of, settle, start_in_background};
use super::cwd_fallback::working_directory;
use super::env::{EnvContext, process_environment};
use super::plan::{PlanContext, PlanEntry, plan_command};
use crate::format::issues::{PluginIssue, Severity};
use crate::hooks::handlers::context::{BackgroundRun, ForegroundRun, RunContext};
use crate::hooks::handlers::timeouts::{TimedEntry, TimeoutPlan, handler_timeout_ms};
use crate::hooks::source_kind::{LabelledEntry, is_plugin_source, plugin_label};
use crate::js::JsValue;
use crate::ports::Signal;
use crate::ports::command_runner::{CommandRun, CommandSpec};
use crate::ports::locations::Locations;
use crate::registry::Port;

/// A registered `command` handler (`HandlerEntry<'command'>`), with the part of it each module it
/// is handed to reads.
#[derive(Clone, Debug)]
pub struct CommandEntry {
    /// The entry as it crossed, for `processEnvironment`.
    pub value: JsValue,
    /// What the plan reads.
    pub plan: PlanEntry,
    /// What the timeout reads.
    pub timed: TimedEntry,
    /// What the missing directory's notice reads.
    pub labelled: LabelledEntry,
    /// What the mode reads.
    pub mode: ModeFields,
}

impl<'de> Deserialize<'de> for CommandEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = JsValue::deserialize(deserializer)?;
        let handler = value.get("handler").cloned().unwrap_or_default();
        Ok(Self {
            plan: PlanEntry::deserialize(value.clone()).map_err(D::Error::custom)?,
            timed: TimedEntry::deserialize(value.clone()).map_err(D::Error::custom)?,
            labelled: LabelledEntry::deserialize(value.clone()).map_err(D::Error::custom)?,
            mode: ModeFields::deserialize(handler).map_err(D::Error::custom)?,
            value,
        })
    }
}

/// The ports of a run this module reads itself.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct CommandPortsRead {
    /// Where the host's directories are.
    pub locations: Port,
    /// Runs the process.
    pub runner: Port,
}

/// The members of a `RunContext` this module reads itself.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandContextRead {
    /// The ports.
    pub ports: CommandPortsRead,
    /// The JSON a hook reads on stdin.
    pub input: String,
    /// What the event adds to the timeouts.
    #[serde(default)]
    pub timeouts: TimeoutPlan,
    /// The engine checks that a plugin hook's directory is still there.
    #[serde(default)]
    pub check_plugin_directory: Option<bool>,
    /// The session's abort.
    pub signal: Signal,
}

/// A `RunContext` as it crosses: a record that holds a function is a port whose members are read
/// one by one, one that holds none is data (docs/RUST.md, "Effects").
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum OwnContext {
    /// The context holds a function.
    Host(Port),
    /// The context is data, its ports nested.
    Data(Box<CommandContextRead>),
}

impl OwnContext {
    async fn read(self) -> CommandContextRead {
        match self {
            Self::Data(read) => *read,
            Self::Host(port) => CommandContextRead {
                ports: port.member("ports").await,
                input: port.member("input").await,
                timeouts: port
                    .member::<Option<TimeoutPlan>>("timeouts")
                    .await
                    .unwrap_or_default(),
                check_plugin_directory: port.member("checkPluginDirectory").await,
                signal: port.member("signal").await,
            },
        }
    }
}

/// The run's `RunContext`, read as each module it is handed to reads it.
#[derive(Clone, Debug)]
pub struct CommandContext {
    own: OwnContext,
    plan: PlanContext,
    run: RunContext,
    env: EnvContext,
}

impl<'de> Deserialize<'de> for CommandContext {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = JsValue::deserialize(deserializer)?;
        Ok(Self {
            own: OwnContext::deserialize(value.clone()).map_err(D::Error::custom)?,
            plan: PlanContext::deserialize(value.clone()).map_err(D::Error::custom)?,
            run: RunContext::deserialize(value.clone()).map_err(D::Error::custom)?,
            env: EnvContext::deserialize(value).map_err(D::Error::custom)?,
        })
    }
}

/// A hook that did not run.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename = "skipped")]
pub struct SkippedRun {
    /// Why.
    pub reason: String,
    /// The line the conversation view shows for a hook that did not run, when the docs give one
    /// (errors.md, "Hook needs quotes around a path with a space").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

/// What a command hook's run is.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CommandRawRun {
    /// It ran, or did not start.
    Foreground(ForegroundRun),
    /// It did not run.
    Skipped(SkippedRun),
    /// It runs in the background.
    Background(BackgroundRun),
}

/// The raw run and the issues the run itself found.
#[derive(Debug, Serialize)]
pub struct HandlerResult {
    /// The run.
    pub run: CommandRawRun,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// A plan with no spec ran nothing: its notice, else the first error, says why.
fn skipped(issues: &[PluginIssue], notice: Option<String>) -> CommandRawRun {
    CommandRawRun::Skipped(match notice {
        None => SkippedRun {
            reason: issues
                .iter()
                .find(|issue| issue.severity == Severity::Error)
                .map_or_else(
                    || "The hook did not run.".to_owned(),
                    |issue| issue.message.clone(),
                ),
            notice: None,
        },
        Some(notice) => SkippedRun {
            reason: notice.clone(),
            notice: Some(notice),
        },
    })
}

/// The run to ask of the port, and the issues finding its directory raised.
struct Prepared {
    run: CommandRun,
    issues: Vec<PluginIssue>,
}

async fn request(
    entry: &CommandEntry,
    spec: CommandSpec,
    exported: &IndexMap<String, String>,
    context: CommandContext,
    own: &CommandContextRead,
) -> Prepared {
    let directory = working_directory(context.run).await;
    let timeout_ms = handler_timeout_ms(&entry.timed, &own.timeouts);
    Prepared {
        issues: directory.issues,
        run: CommandRun {
            spec,
            cwd: directory.cwd,
            env: process_environment(&entry.value, exported, &context.env.read().await).await,
            stdin: Some(own.input.clone()),
            timeout_ms,
            merge_stderr: None,
            signal: own.signal.clone(),
        },
    }
}

/// "A plugin's hook fails with `Failed to run: Plugin directory does not exist: <path> (<plugin> — run
/// /plugin to reinstall)`, and the hook doesn't run, when the directory your session loaded the
/// plugin's hooks from is gone from disk" (plugins/troubleshooting.md). Judgment (spec/judgments.md):
/// the engine turns the check on (`checkPluginDirectory`) and a host that never re-reads its plugins
/// can turn it off, since it costs a directory lookup per plugin hook run; the docs' "shown once per
/// session for each hook event and command" is the host's to do, since it holds the session.
async fn missing_plugin_directory(
    entry: &CommandEntry,
    context: &CommandContextRead,
) -> Option<String> {
    let LabelledEntry { plugin_id, source } = &entry.labelled;
    if context.check_plugin_directory != Some(true) || !is_plugin_source(source) {
        return None;
    }
    let plugin_id = plugin_id.as_deref()?;
    let locations = context.ports.locations;
    let root = locations.plugin_root(plugin_id).await;
    let found = locations
        .existing_directory(std::slice::from_ref(&root), Some(&context.signal))
        .await;
    match found {
        None => Some(format!(
            "Failed to run: Plugin directory does not exist: {root} ({} — run /plugin to reinstall)",
            plugin_label(&entry.labelled).unwrap_or_else(|| plugin_id.to_owned())
        )),
        Some(_) => None,
    }
}

/// Runs one `command` hook.
pub async fn run_command(entry: &CommandEntry, context: CommandContext) -> HandlerResult {
    let own = context.own.clone().read().await;
    if let Some(message) = missing_plugin_directory(entry, &own).await {
        return HandlerResult {
            run: CommandRawRun::Foreground(ForegroundRun::NotStarted { message }),
            issues: Vec::new(),
        };
    }
    let plan = plan_command(&entry.plan, &context.plan.clone().read().await).await;
    if let Some(message) = plan.not_started {
        return HandlerResult {
            run: CommandRawRun::Foreground(ForegroundRun::NotStarted { message }),
            issues: plan.issues,
        };
    }
    let Some(spec) = plan.spec else {
        return HandlerResult {
            run: skipped(&plan.issues, plan.notice),
            issues: plan.issues,
        };
    };
    let exported = plan.exported.unwrap_or_default();
    let prepared = request(entry, spec, &exported, context, &own).await;
    let issues = [plan.issues, prepared.issues].concat();
    let runner = own.ports.runner;
    let mode = MODES.of(mode_of(&entry.mode));
    let run = if mode.background {
        CommandRawRun::Background(start_in_background(runner, prepared.run, mode))
    } else {
        CommandRawRun::Foreground(settle(&runner, prepared.run).await)
    };
    HandlerResult { run, issues }
}

crate::registry::exports! {
    "hooks/handlers/command/run-command" {
        "runCommand" => async ok |entry: CommandEntry, context: CommandContext|
            run_command(&entry, context).await;
    }
}
