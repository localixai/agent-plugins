//! Port of `src/updates/command-source-rerun.ts`, switched.
//!
//! When the command of a `command` source runs again, and what follows a run (plugins/loading.md,
//! "When a command source re-runs"; plugins/host-marketplace.md, "Change the command of a command
//! source"). Pure: the host runs the command (`runCommandSource`); the kit says which runs happen,
//! which are skipped and why, what the user is asked, and what an Errors-tab entry says.

use serde::{Deserialize, Serialize};

use super::env_flags::{DISABLE_NONESSENTIAL_TRAFFIC, is_set};
use crate::format::issues::{PluginIssue, error};
use crate::marketplace::update_detection::{UpdateOutcome, is_update};
use crate::ports::environment::Environment;
use crate::registry::Port;

/// What starts a decision: an explicit install or update, the start of a session, `/reload-plugins`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RerunEvent {
    /// An explicit install.
    Install,
    /// An explicit update.
    Update,
    /// The start of a session.
    SessionStart,
    /// `/reload-plugins`.
    ReloadPlugins,
}

/// `RERUN_EVENTS`.
pub const RERUN_EVENTS: [RerunEvent; 4] = [
    RerunEvent::Install,
    RerunEvent::Update,
    RerunEvent::SessionStart,
    RerunEvent::ReloadPlugins,
];

/// The runs the docs list: "Every time you install or update", "Once per session", "installed
/// version is missing".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunKind {
    /// "Every time you install".
    Install,
    /// "or update".
    Update,
    /// "Once per session".
    OncePerSession,
    /// "installed version is missing".
    MissingVersion,
}

/// `RUN_KINDS`.
pub const RUN_KINDS: [RunKind; 4] = [
    RunKind::Install,
    RunKind::Update,
    RunKind::OncePerSession,
    RunKind::MissingVersion,
];

/// How a command source's output is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandMode {
    /// Copied.
    Copy,
    /// Linked.
    Link,
}

/// The command and `mode` a user accepted, or that the catalog declares now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDeclaration {
    /// The command.
    pub command: String,
    /// Its mode.
    pub mode: CommandMode,
}

/// A command-sourced plugin.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPlugin {
    /// `name@marketplace`.
    pub id: String,
    /// Whether it is enabled.
    pub enabled: bool,
    /// The installed version's directory exists in the plugin cache.
    pub installed_version_in_cache: bool,
    /// What the marketplace entry declares now.
    pub declared: CommandDeclaration,
    /// What the user accepted when they installed or last updated; none for a plugin never
    /// installed.
    #[serde(default)]
    pub accepted: Option<CommandDeclaration>,
}

/// Why a run is skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunSkip {
    /// The command was not accepted.
    CommandNotAccepted,
    /// `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is set.
    NonessentialTrafficDisabled,
}

/// "Claude Code shows them the new command and asks them to accept it."
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Acceptance {
    /// Whether the user is asked.
    pub ask: bool,
    /// The command shown, when asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub showing: Option<String>,
}

/// "An entry in the `/plugin` Errors tab: the entry shows the new command and the `claude plugin
/// update` command to run."
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorsTabEntry {
    /// The plugin.
    pub plugin: String,
    /// Its new command.
    pub command: String,
    /// Its mode.
    pub mode: CommandMode,
    /// The command to run.
    pub update_command: String,
    /// The entry as an issue.
    pub issue: PluginIssue,
}

/// A run skipped, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Skipped {
    /// The run.
    pub run: RunKind,
    /// Why.
    pub reason: RunSkip,
}

/// The runs an event brings about.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RerunPlan {
    /// The runs that start now.
    pub runs: Vec<RunKind>,
    /// The runs held until the user accepts the command shown by `acceptance`: "has to accept the
    /// new command before Claude Code runs it".
    pub runs_after_acceptance: Vec<RunKind>,
    /// The runs skipped.
    pub skipped: Vec<Skipped>,
    /// Set for an install or update: whether the user must accept the command first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<Acceptance>,
    /// The Errors-tab entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors_tab: Option<ErrorsTabEntry>,
}

/// A plan as `runsAfterAnswer` reads it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRuns {
    /// The runs that start now.
    pub runs: Vec<RunKind>,
    /// The runs held until the user accepts.
    pub runs_after_acceptance: Vec<RunKind>,
}

/// Whether a run is one of "the two background runs" the traffic variable skips (`BACKGROUND`).
const fn background(run: RunKind) -> bool {
    match run {
        RunKind::Install | RunKind::Update => false,
        RunKind::OncePerSession | RunKind::MissingVersion => true,
    }
}

/// Which runs an event brings about; the background ones only "for each enabled" plugin (`DUE`).
fn due(event: RerunEvent, plugin: &CommandPlugin) -> Vec<RunKind> {
    match event {
        RerunEvent::Install => vec![RunKind::Install],
        RerunEvent::Update => vec![RunKind::Update],
        RerunEvent::SessionStart => {
            if !plugin.enabled {
                return Vec::new();
            }
            if plugin.installed_version_in_cache {
                vec![RunKind::OncePerSession]
            } else {
                vec![RunKind::OncePerSession, RunKind::MissingVersion]
            }
        }
        RerunEvent::ReloadPlugins => {
            if plugin.enabled && !plugin.installed_version_in_cache {
                vec![RunKind::MissingVersion]
            } else {
                Vec::new()
            }
        }
    }
}

/// The events that look at the catalog the way a session does, and so show the Errors-tab entry
/// (`SHOWS_ERRORS_TAB`).
const fn shows_errors_tab(event: RerunEvent) -> bool {
    match event {
        RerunEvent::Install | RerunEvent::Update => false,
        RerunEvent::SessionStart | RerunEvent::ReloadPlugins => true,
    }
}

fn is_changed(plugin: &CommandPlugin) -> bool {
    plugin.accepted.as_ref().is_none_or(|accepted| {
        accepted.command != plugin.declared.command || accepted.mode != plugin.declared.mode
    })
}

fn asking(plugin: &CommandPlugin) -> Acceptance {
    Acceptance {
        ask: true,
        showing: Some(plugin.declared.command.clone()),
    }
}

/// An install always shows its command and asks; an update asks only when the command or mode it
/// would run is not the accepted one ("Claude Code runs only the exact command a user accepted")
/// (`ACCEPTANCE`).
fn acceptance(event: RerunEvent, plugin: &CommandPlugin) -> Option<Acceptance> {
    match event {
        RerunEvent::Install => Some(asking(plugin)),
        RerunEvent::Update => Some(if is_changed(plugin) {
            asking(plugin)
        } else {
            Acceptance {
                ask: false,
                showing: None,
            }
        }),
        RerunEvent::SessionStart | RerunEvent::ReloadPlugins => None,
    }
}

fn entry_for(plugin: &CommandPlugin) -> ErrorsTabEntry {
    let CommandDeclaration { command, mode } = &plugin.declared;
    let update_command = format!("claude plugin update {}", plugin.id);
    let mode_text = match mode {
        CommandMode::Copy => "copy",
        CommandMode::Link => "link",
    };
    ErrorsTabEntry {
        plugin: plugin.id.clone(),
        command: command.clone(),
        mode: *mode,
        // Judgment (spec/judgments.md): the docs say what the entry shows, not how it is worded.
        issue: error(
            &format!("plugins.{}", plugin.id),
            &format!(
                "The install command of {} changed to `{command}` (mode: {mode_text}) and needs your acceptance: run `{update_command}` in a terminal",
                plugin.id
            ),
        ),
        update_command,
    }
}

/// A background run is stopped by an unaccepted command first, then by the traffic variable.
async fn verdict(run: RunKind, plugin: &CommandPlugin, env: &dyn Environment) -> Option<RunSkip> {
    if !background(run) {
        return None;
    }
    if is_changed(plugin) {
        return Some(RunSkip::CommandNotAccepted);
    }
    if is_set(env, DISABLE_NONESSENTIAL_TRAFFIC).await {
        Some(RunSkip::NonessentialTrafficDisabled)
    } else {
        None
    }
}

/// The runs `event` brings about for one command-sourced plugin. `DISABLE_AUTOUPDATER` and the
/// marketplace's `autoUpdate` have no part in it: only the traffic variable and an unaccepted
/// command stop the background runs, and never an install or update.
pub async fn command_source_runs(
    event: RerunEvent,
    plugin: &CommandPlugin,
    env: &dyn Environment,
) -> RerunPlan {
    let mut verdicts = Vec::new();
    for run in due(event, plugin) {
        verdicts.push((run, verdict(run, plugin, env).await));
    }
    let acceptance = acceptance(event, plugin);
    let shows_entry = shows_errors_tab(event) && plugin.enabled && is_changed(plugin);
    let due: Vec<RunKind> = verdicts
        .iter()
        .filter(|(_, skip)| skip.is_none())
        .map(|(run, _)| *run)
        .collect();
    let asks = acceptance.as_ref().is_some_and(|each| each.ask);
    RerunPlan {
        runs: if asks { Vec::new() } else { due.clone() },
        runs_after_acceptance: if asks { due } else { Vec::new() },
        skipped: verdicts
            .iter()
            .filter_map(|(run, skip)| skip.map(|reason| Skipped { run: *run, reason }))
            .collect(),
        acceptance,
        errors_tab: shows_entry.then(|| entry_for(plugin)),
    }
}

/// The runs that go ahead once the user has answered the question of `plan.acceptance`: a declined
/// command never runs.
#[must_use]
pub fn runs_after_answer(plan: PlanRuns, accepted: bool) -> Vec<RunKind> {
    if accepted {
        plan.runs
            .into_iter()
            .chain(plan.runs_after_acceptance)
            .collect()
    } else {
        plan.runs
    }
}

/// What a finished run leaves to do with the version it produced.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    /// The version `installed_plugins.json` records; none for a plugin never installed.
    #[serde(default)]
    pub installed_version: Option<String>,
    /// The version the hash of the command's output gives (`resolvePluginVersion`).
    pub computed_version: String,
    /// A running interactive session exists to reload into.
    pub interactive: bool,
    /// Reloading in place "would invalidate the session's prompt cache".
    pub reload_invalidates_prompt_cache: bool,
}

/// What follows a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FollowUp {
    /// Nothing.
    Nothing,
    /// Reload in place.
    Reload,
    /// Ask for `/reload-plugins`.
    AskReloadPlugins,
    /// At the next launch.
    NextLaunch,
}

/// What a finished run leaves to do: an install of the new version and its follow-up.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterRun {
    /// Whether the new version is installed.
    pub install: bool,
    /// What follows.
    pub follow_up: FollowUp,
    /// "You see a notification that the plugin was reloaded." The reload switches what
    /// `/reload-plugins` switches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<&'static str>,
}

/// The state of the session a run finished in.
#[derive(Clone, Copy)]
enum SessionState {
    InteractiveCacheSafe,
    InteractiveCacheCost,
    NotInteractive,
}

/// The follow-up of an installed new version, by the state of the session (`FOLLOW_UP`).
const fn follow_up(state: SessionState) -> AfterRun {
    match state {
        SessionState::InteractiveCacheSafe => AfterRun {
            install: true,
            follow_up: FollowUp::Reload,
            notice: Some("plugin-reloaded"),
        },
        SessionState::InteractiveCacheCost => AfterRun {
            install: true,
            follow_up: FollowUp::AskReloadPlugins,
            notice: None,
        },
        SessionState::NotInteractive => AfterRun {
            install: true,
            follow_up: FollowUp::NextLaunch,
            notice: None,
        },
    }
}

const fn session_state(result: &RunResult) -> SessionState {
    if !result.interactive {
        return SessionState::NotInteractive;
    }
    if result.reload_invalidates_prompt_cache {
        SessionState::InteractiveCacheCost
    } else {
        SessionState::InteractiveCacheSafe
    }
}

/// "When the command's hashed output has changed, Claude Code installs the result as a new version
/// and reloads it in the running interactive session"; if that "would invalidate the session's
/// prompt cache" it prompts for `/reload-plugins` instead. A session with nothing to reload into
/// loads the new version at its next launch (judgment, spec/judgments.md).
#[must_use]
pub fn after_command_run(result: &RunResult) -> AfterRun {
    if is_update(result.installed_version.as_deref(), &result.computed_version)
        == UpdateOutcome::Unchanged
    {
        return AfterRun {
            install: false,
            follow_up: FollowUp::Nothing,
            notice: None,
        };
    }
    follow_up(session_state(result))
}

crate::registry::exports! {
    "updates/command-source-rerun" {
        "RERUN_EVENTS" => ok || RERUN_EVENTS;
        "RUN_KINDS" => ok || RUN_KINDS;
        "commandSourceRuns" => async ok |event: RerunEvent, plugin: CommandPlugin, env: Port|
            command_source_runs(event, &plugin, &env).await;
        "runsAfterAnswer" => ok |plan: PlanRuns, accepted: bool| runs_after_answer(plan, accepted);
        "afterCommandRun" => ok |result: RunResult| after_command_run(&result);
    }
}
