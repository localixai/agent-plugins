//! The core's counterpart of `src/hooks/handlers/context.ts` at sha256:fb48d5f424fe, which stays
//! TypeScript: types only, so there is nothing to call across the boundary (docs/RUST.md, "What
//! stays TypeScript"). Only the runs the ported modules make are here: a command's run, and one
//! moved to the background (`hooks/handlers/command/async.rs`), and the part of a `RunContext` the
//! ported handlers read.

use serde::{Deserialize, Serialize};

use super::remote::http_policy::HttpHookPolicy;
use super::timeouts::TimeoutPlan;
use crate::ports::Signal;
use crate::registry::{CorePromise, Port};

/// A run that ended: the exit code and what the process wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitedRun {
    /// The exit code.
    pub exit_code: i32,
    /// What it wrote to standard output.
    pub stdout: String,
    /// What it wrote to standard error.
    pub stderr: String,
    /// The hook's own process ended while a process it started still held its stdout or stderr
    /// open (hooks.md, "Hooks that start a background process"): the capture is suspect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_held_open: Option<bool>,
}

/// A run that ended another way. `timed-out` discards the output ("Claude Code cancels a hook
/// that reaches its timeout, discarding the hook's output"); `not-started` is a program that did
/// not start (a missing script); `cancelled` is a process killed by the session (an abort).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ForegroundRun {
    /// It exited.
    #[serde(rename = "exited")]
    Exited(ExitedRun),
    /// It reached its timeout.
    #[serde(rename = "timed-out")]
    TimedOut,
    /// It did not start.
    #[serde(rename = "not-started")]
    NotStarted {
        /// Why.
        message: String,
    },
    /// The session killed it.
    #[serde(rename = "cancelled")]
    Cancelled,
}

/// What a background hook reports once its process ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundResult {
    /// How the run ended.
    pub run: ForegroundRun,
    /// `asyncRewake` only: the text that wakes Claude, the stderr (stdout when stderr is empty)
    /// of a run that exited with code 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wake: Option<String>,
}

/// A hook running in the background: the engine goes on, and reads `done` when it settles.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename = "background")]
pub struct BackgroundRun {
    /// What it reports once its process ends.
    pub done: CorePromise<BackgroundResult>,
}

/// The ports of a run (`RunPorts`) that the ported handlers use, each the host's object.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct RunPortsRead {
    /// Where the host's directories are.
    pub locations: Port,
    /// The host's environment variables.
    pub environment: Port,
    /// `http` hooks; absent, a hook of that type does not run.
    #[serde(default)]
    pub http: Option<Port>,
    /// `mcp_tool` hooks.
    #[serde(default)]
    pub mcp: Option<Port>,
    /// Time: an `mcp_tool` hook subtracts what it waited from its `timeout`.
    #[serde(default)]
    pub clock: Option<Port>,
}

/// The members of a `RunContext` the ported handlers read.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunContextRead {
    /// The ports.
    pub ports: RunPortsRead,
    /// The JSON a hook reads on stdin.
    pub input: String,
    /// Where Claude is working now.
    pub cwd: String,
    /// The directory the session started in.
    #[serde(default)]
    pub start_directory: Option<String>,
    /// The system temp directory.
    #[serde(default)]
    pub temp_directory: Option<String>,
    /// What the event adds to the timeouts.
    #[serde(default)]
    pub timeouts: TimeoutPlan,
    /// The merged `allowedHttpHookUrls` and `httpHookAllowedEnvVars`.
    #[serde(default)]
    pub http_policy: Option<HttpHookPolicy>,
    /// The session's abort.
    pub signal: Signal,
}

/// A `RunContext` as it crosses: a record that holds a function (`userConfigOf`) is a port whose
/// members are read as they are needed, one that holds none is data (docs/RUST.md, "Effects").
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RunContext {
    /// The context holds a function.
    Host(Port),
    /// The context is data, its ports nested.
    Data(Box<RunContextRead>),
}

impl RunContext {
    /// The members the ported handlers read.
    pub async fn read(self) -> RunContextRead {
        match self {
            Self::Data(read) => *read,
            Self::Host(port) => RunContextRead {
                ports: port.member::<RunPortsRead>("ports").await,
                input: port.member::<String>("input").await,
                cwd: port.member::<String>("cwd").await,
                start_directory: port.member::<Option<String>>("startDirectory").await,
                temp_directory: port.member::<Option<String>>("tempDirectory").await,
                timeouts: port
                    .member::<Option<TimeoutPlan>>("timeouts")
                    .await
                    .unwrap_or_default(),
                http_policy: port.member::<Option<HttpHookPolicy>>("httpPolicy").await,
                signal: port.member::<Signal>("signal").await,
            },
        }
    }
}
