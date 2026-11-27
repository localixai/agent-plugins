//! Port of `src/marketplace/command-source.ts`, switched.
//!
//! Running a `command` source (plugins/marketplace-reference.md, "command plugin source"): the
//! command goes through the `CommandRunner` port, from the user's home directory; what it printed
//! and the directory it names are judged here, the copy or the link itself is the host's. Every
//! failure the docs list is a `PluginIssue` on `source`; nothing throws.

use serde::{Deserialize, Serialize};

use super::command_directory::{content_issue, copy_limit_issue, link_issue};
use super::command_mode::{LINKS_NEED_POSIX, link_on_windows_issue};
use super::command_output::{PrintedPath, contains_session, read_printed_path};
use super::install_mode::session_directory_issue;
use super::ports::PluginDirectories;
use super::types::{CommandFetchPlan, CommandMode, CommandShell};
use crate::format::issues::{PluginIssue, error};
use crate::format::path_text::PathPlatform;
use crate::js::number_to_string;
use crate::ports::Signal;
use crate::ports::command_runner::{CommandResult, CommandRun, CommandRunner, CommandSpec};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::registry::Port;

/// The host's ports a run needs (`CommandSourcePorts`).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct CommandSourcePorts {
    /// Runs the command.
    pub runner: Port,
    /// The environment the command inherits ("Claude Code's subprocess environment").
    pub environment: Port,
    /// `homeDir()` is the directory the command runs from.
    pub locations: Port,
    /// Opens the printed directory.
    pub directories: Port,
}

/// Where the command runs (`CommandSourceSetting`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSourceSetting {
    /// The platform.
    pub platform: PathPlatform,
    /// The directory the session was started in: a source may not print it or a parent of it.
    #[serde(default)]
    pub session_cwd: Option<String>,
}

/// What a successful run leaves for the host: the directory to copy or link, as read-only files.
#[derive(Clone, Debug, Serialize)]
pub struct FetchedCommandPlugin {
    /// The printed directory.
    pub directory: String,
    /// Its files, the host's.
    pub files: Port,
    /// How it is installed.
    pub mode: CommandMode,
}

/// The fetched plugin, or the issue that refuses it.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum CommandSourceResult {
    /// The fetched plugin.
    Fetched(FetchedCommandPlugin),
    /// The issue.
    Refused(PluginIssue),
}

/// `sh -c <command>`, or `cmd.exe /c <command>`: the exec form of the shell the docs name.
#[must_use]
pub fn shells(shell: CommandShell, command: &str) -> CommandSpec {
    match shell {
        CommandShell::Sh => CommandSpec::Exec {
            command: "sh".to_owned(),
            args: vec!["-c".to_owned(), command.to_owned()],
        },
        CommandShell::Cmd => CommandSpec::Exec {
            command: "cmd.exe".to_owned(),
            args: vec!["/c".to_owned(), command.to_owned()],
        },
    }
}

/// `LINKS_NEED_POSIX[mode]`.
const fn links_need_posix(mode: CommandMode) -> bool {
    match mode {
        CommandMode::Copy => LINKS_NEED_POSIX.copy,
        CommandMode::Link => LINKS_NEED_POSIX.link,
    }
}

fn problem(message: &str) -> PluginIssue {
    error("source.command", message)
}

/// The failures of the run itself: did not start, timed out, exited non-zero.
fn run_issue(result: &CommandResult, plan: &CommandFetchPlan) -> Option<PluginIssue> {
    if let Some(start_error) = &result.start_error {
        return Some(problem(&format!(
            "The command could not be run: {start_error}"
        )));
    }
    if result.timed_out {
        return Some(problem(&format!(
            "The command ran longer than its timeout of {} seconds",
            number_to_string(plan.timeout_ms / 1000.0)
        )));
    }
    if result.exit_code != Some(0) {
        let code = result.exit_code.map_or_else(
            || "was stopped before it exited".to_owned(),
            |exit_code| format!("exited with code {exit_code}"),
        );
        return Some(problem(&format!("The command {code}; it must exit 0")));
    }
    None
}

async fn execute(
    plan: &CommandFetchPlan,
    ports: &CommandSourcePorts,
    signal: &Signal,
) -> CommandResult {
    let run = CommandRun {
        spec: shells(plan.shell, &plan.command),
        cwd: ports.locations.home_dir().await,
        env: ports.environment.snapshot().await,
        stdin: None,
        timeout_ms: Some(plan.timeout_ms),
        merge_stderr: None,
        signal: signal.clone(),
    };
    ports.runner.run(run).await
}

/// The checks of the directory itself, in the order the docs list the failures.
async fn directory_issue(files: &Port, mode: CommandMode, signal: &Signal) -> Option<PluginIssue> {
    if let Some(missing) = content_issue(files, signal).await {
        return Some(missing);
    }
    if links_need_posix(mode) {
        link_issue(files, signal).await
    } else {
        copy_limit_issue(files, signal).await
    }
}

/// Runs the command and judges what it printed; the fetched plugin, or the issue that refuses it.
pub async fn run_command_source(
    plan: &CommandFetchPlan,
    ports: &CommandSourcePorts,
    setting: &CommandSourceSetting,
    signal: &Signal,
) -> CommandSourceResult {
    use CommandSourceResult::Refused;
    if setting.platform == PathPlatform::Windows && links_need_posix(plan.mode) {
        return Refused(link_on_windows_issue());
    }
    let result = execute(plan, ports, signal).await;
    if let Some(failed) = run_issue(&result, plan) {
        return Refused(failed);
    }
    let path = match read_printed_path(&result.stdout, setting.platform) {
        PrintedPath::Problem { problem: message } => return Refused(problem(&message)),
        PrintedPath::Path { path } => path,
    };
    if contains_session(&path, setting.session_cwd.as_deref(), setting.platform) {
        return Refused(session_directory_issue(&path));
    }
    let Some(files) = ports.directories.open(&path, signal).await else {
        return Refused(error(
            "source",
            &format!("The printed directory \"{path}\" does not exist"),
        ));
    };
    match directory_issue(&files, plan.mode, signal).await {
        Some(refused) => Refused(refused),
        None => CommandSourceResult::Fetched(FetchedCommandPlugin {
            directory: path,
            files,
            mode: plan.mode,
        }),
    }
}

/// `{ signal }`, the options of `runCommandSource`.
#[derive(Debug, Deserialize)]
pub struct WithSignal {
    /// The run's signal.
    pub signal: Signal,
}

crate::registry::exports! {
    "marketplace/command-source" {
        "runCommandSource" => async ok
            |plan: CommandFetchPlan, ports: CommandSourcePorts, setting: CommandSourceSetting, options: WithSignal|
            run_command_source(&plan, &ports, &setting, &options.signal).await;
    }
}
