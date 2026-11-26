//! Port of `src/hooks/core/run.ts`, switched.
//!
//! One invocation of a core handler (core §7.3.3; Agent Plugins Extended §18.1.2, "Failures, limits
//! and presentation"): a fresh process per invocation through the `CommandRunner` port, in exec form
//! (`command` one token, `args` separate, no shell), the plugin root as working directory, the core
//! input as one JSON object on stdin, stderr kept apart from stdout so it cannot make the output
//! invalid. The time limit is a Claude Code `command` hook's default on the same event (J18.5): 30
//! seconds on `UserPromptSubmit`, 600 seconds on the other four.

use serde::{Deserialize, Serialize};

use super::output::{CoreReading, read_core_run};
use super::types::CoreRun;
use crate::format::core::hooks::CoreHookEvent;
use crate::format::core::subprocess::CoreProcess;
use crate::js::{JsObject, JsValue, json_stringify};
use crate::ports::Signal;
use crate::ports::command_runner::{CommandRun, CommandRunner, CommandSpec};
use crate::registry::Port;

const SECONDS: f64 = 1000.0;

fn core_timeout_ms(event: CoreHookEvent) -> f64 {
    match event {
        CoreHookEvent::SessionStart => 600.0 * SECONDS,
        CoreHookEvent::UserPromptSubmit => 30.0 * SECONDS,
        CoreHookEvent::PreToolUse => 600.0 * SECONDS,
        CoreHookEvent::PostToolUse => 600.0 * SECONDS,
        CoreHookEvent::Stop => 600.0 * SECONDS,
    }
}

/// What one run is made of.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreInvocation {
    /// The handler.
    pub handler_id: String,
    /// The plugin.
    pub plugin_id: String,
    /// The event.
    pub event: CoreHookEvent,
    /// What a host launches.
    pub process: CoreProcess,
    /// The handler as written in `hooks.json`: its `command` and `args` joined by spaces (J18.22).
    pub label: String,
    /// The core input.
    pub input: JsObject,
}

/// Runs the handler once and reads what it came to; the port's own failure is a failed run too.
pub async fn run_core_handler(
    runner: &(impl CommandRunner + ?Sized),
    invocation: &CoreInvocation,
    signal: Signal,
) -> CoreRun {
    let event = invocation.event;
    let launch = &invocation.process;
    let timeout_ms = core_timeout_ms(event);
    let run = runner
        .run(CommandRun {
            spec: CommandSpec::Exec {
                command: launch.command.clone(),
                args: launch.args.clone(),
            },
            cwd: launch.cwd.clone(),
            env: launch.env.clone(),
            stdin: Some(
                json_stringify(&JsValue::Object(invocation.input.clone())).unwrap_or_default(),
            ),
            timeout_ms: Some(timeout_ms),
            merge_stderr: None,
            signal,
        })
        .await;
    let reading = read_core_run(event, &run, timeout_ms);
    let (result, failure) = match reading {
        CoreReading::Result { result } => (result, None),
        CoreReading::Failure { failure } => (None, Some(failure)),
    };
    CoreRun {
        handler_id: invocation.handler_id.clone(),
        plugin_id: invocation.plugin_id.clone(),
        label: invocation.label.clone(),
        timeout_ms,
        exit_code: run.exit_code.map(f64::from),
        timed_out: run.timed_out.then_some(true),
        result,
        failure,
    }
}

crate::registry::exports! {
    "hooks/core/run" {
        "runCoreHandler" => async ok |runner: Port, invocation: CoreInvocation, signal: Signal|
            run_core_handler(&runner, &invocation, signal).await;
    }
}
