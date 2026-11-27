//! The core's counterpart of `src/hooks/handlers/command/async.ts` at sha256:d260ef9621dd, which stays
//! TypeScript: types only now (a runner's `HeldOpenResult`), since every caller of its runs is in
//! the core.
//!
//! Background command hooks (hooks.md, "Run hooks in the background" and "Command hook fields").
//! `async: true` "runs in the background without blocking"; `asyncRewake: true` "runs in the
//! background and wakes Claude on exit code 2", and "implies async" (the SDK type). The timeout is
//! not enforced on an `async` hook but is on an `asyncRewake` one: that is decided where the
//! limit is computed (`handlerTimeoutMs`), so the request here already carries it or not.

use serde::{Deserialize, Serialize};

use crate::hooks::handlers::context::{BackgroundResult, BackgroundRun, ExitedRun, ForegroundRun};
use crate::js;
use crate::ports::command_runner::{CommandResult, CommandRun, CommandRunner};
use crate::registry::CorePromise;

/// How a command hook runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunMode {
    /// The engine waits for the process.
    Foreground,
    /// In the background.
    Async,
    /// In the background, and an exit with code 2 wakes Claude.
    Rewake,
}

/// What a mode does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mode {
    /// The engine does not wait for the process.
    pub background: bool,
    /// An exit with code 2 wakes Claude.
    pub wakes: bool,
}

/// Each mode's [`Mode`], by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Modes {
    /// `foreground`.
    pub foreground: Mode,
    /// `async`.
    #[serde(rename = "async")]
    pub r#async: Mode,
    /// `rewake`.
    pub rewake: Mode,
}

impl Modes {
    /// `MODES[mode]`.
    #[must_use]
    pub const fn of(&self, mode: RunMode) -> Mode {
        match mode {
            RunMode::Foreground => self.foreground,
            RunMode::Async => self.r#async,
            RunMode::Rewake => self.rewake,
        }
    }
}

/// The modes.
pub const MODES: Modes = Modes {
    foreground: Mode {
        background: false,
        wakes: false,
    },
    r#async: Mode {
        background: true,
        wakes: false,
    },
    rewake: Mode {
        background: true,
        wakes: true,
    },
};

/// The two fields of a command handler `modeOf` reads.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeFields {
    /// `async`.
    #[serde(default, rename = "async")]
    pub r#async: Option<bool>,
    /// `asyncRewake`.
    #[serde(default)]
    pub async_rewake: Option<bool>,
}

/// The mode a handler runs in.
#[must_use]
pub fn mode_of(handler: &ModeFields) -> RunMode {
    if handler.async_rewake == Some(true) {
        RunMode::Rewake
    } else if handler.r#async == Some(true) {
        RunMode::Async
    } else {
        RunMode::Foreground
    }
}

const WAKE_EXIT_CODE: i32 = 2;

/// "The hook's stderr, or stdout if stderr is empty, is shown to Claude as a system reminder."
/// Judgment (spec/judgments.md): stderr of only whitespace is empty, and when the text that is left
/// is blank too there is nothing to say, so Claude is not woken.
fn wake_message(run: &ForegroundRun) -> Option<String> {
    let ForegroundRun::Exited(exited) = run else {
        return None;
    };
    if exited.exit_code != WAKE_EXIT_CODE {
        return None;
    }
    let text = if js::trim(&exited.stderr).is_empty() {
        &exited.stdout
    } else {
        &exited.stderr
    };
    if js::trim(text).is_empty() {
        None
    } else {
        Some(text.clone())
    }
}

/// What a runner answers when it can tell that a process the hook started still holds the output
/// open: "Claude Code finishes reading the hook's output shortly after the hook's own process
/// exits", and such a capture is suspect (docs/API.md, "Ports"). In the TypeScript a runner
/// declares its answer as `HeldOpenResult`; in the core the field is
/// [`CommandResult::output_held_open`].
fn held_open(result: &CommandResult) -> bool {
    result.output_held_open == Some(true)
}

/// The port's answer as the run the interpreter reads. A start error under an aborted signal is a
/// cancelled run (Node's `spawn({ signal })` answers an abort with an error, not an exit), and a
/// signal aborted before the start runs nothing; a run that exited keeps its exit.
pub async fn settle(runner: &(impl CommandRunner + ?Sized), request: CommandRun) -> ForegroundRun {
    if request.signal.is_aborted() {
        return ForegroundRun::Cancelled;
    }
    let signal = request.signal.clone();
    let result = runner.run(request).await;
    if let Some(start_error) = result.start_error.as_ref() {
        return if signal.is_aborted() {
            ForegroundRun::Cancelled
        } else {
            ForegroundRun::NotStarted {
                message: start_error.clone(),
            }
        };
    }
    if result.timed_out {
        return ForegroundRun::TimedOut;
    }
    let Some(exit_code) = result.exit_code else {
        return ForegroundRun::Cancelled;
    };
    ForegroundRun::Exited(ExitedRun {
        exit_code,
        output_held_open: held_open(&result).then_some(true),
        stdout: result.stdout,
        stderr: result.stderr,
    })
}

/// Starts the process and returns at once; `done` settles when it ends.
pub fn start_in_background<R: CommandRunner + 'static>(
    runner: R,
    request: CommandRun,
    mode: Mode,
) -> BackgroundRun {
    let done = CorePromise::new(async move {
        let run = settle(&runner, request).await;
        let wake = if mode.wakes { wake_message(&run) } else { None };
        BackgroundResult { run, wake }
    });
    BackgroundRun { done }
}

#[cfg(test)]
mod tests;
