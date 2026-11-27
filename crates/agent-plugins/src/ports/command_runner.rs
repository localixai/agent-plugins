//! The core's counterpart of `src/ports/command-runner.ts` at sha256:dc111b95ff57, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! Running a command: the hooks' command handlers, a skill's `` !`cmd` `` expansion, a helper
//! that prints headers. The crate never builds a process line: it says which form to run, and the
//! host runs it. Timeouts are computed by the crate and enforced by the port.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{BoxFuture, Signal};

/// The shell a `shell` command runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellKind {
    /// `bash -c`.
    Bash,
    /// PowerShell's `-Command`.
    Powershell,
}

/// What to run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "form", rename_all = "lowercase")]
pub enum CommandSpec {
    /// A program with its arguments, run without a shell.
    Exec {
        /// The program.
        command: String,
        /// Its arguments.
        args: Vec<String>,
    },
    /// A command line, run by a shell.
    Shell {
        /// The command line.
        command: String,
        /// The shell that reads it.
        shell: ShellKind,
    },
}

/// One run of a command.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRun {
    /// What to run.
    pub spec: CommandSpec,
    /// The working directory, a host path.
    pub cwd: String,
    /// The whole environment of the process.
    pub env: IndexMap<String, String>,
    /// Text written to its standard input, then closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    /// After how long the host stops it and answers `timed_out`, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<f64>,
    /// Whether standard error is read into `stdout`, in order; absent is no.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_stderr: Option<bool>,
    /// Its cancellation.
    pub signal: Signal,
}

/// How a run ended.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    /// The exit code; `None` when the process was stopped by a signal or never started.
    pub exit_code: Option<i32>,
    /// What it wrote to standard output.
    pub stdout: String,
    /// What it wrote to standard error.
    pub stderr: String,
    /// Whether the host stopped it at its timeout.
    pub timed_out: bool,
    /// Why it could not start, when it could not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_error: Option<String>,
    /// A process the command started still held its output open when it ended, so the capture is
    /// suspect: a field a runner may add (`HeldOpenResult`, `hooks/handlers/command/async`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_held_open: Option<bool>,
}

/// The host's way to run commands.
pub trait CommandRunner: Send + Sync {
    /// Runs one command to its end.
    fn run(&self, run: CommandRun) -> BoxFuture<'_, CommandResult>;
}
