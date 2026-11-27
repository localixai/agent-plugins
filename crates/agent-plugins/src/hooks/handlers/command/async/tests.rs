//! A background run as a Rust host awaits it: its result wakes Claude on exit 2 for `rewake`
//! only. And `settle`: a signal aborted before the start runs nothing.

use std::future::ready;
use std::sync::Mutex;

use serde::Deserialize;

use super::{Mode, settle, start_in_background};
use crate::hooks::handlers::context::{BackgroundResult, ExitedRun, ForegroundRun};
use crate::js::JsValue;
use crate::js::json_parse::json_parse;
use crate::ports::command_runner::{CommandResult, CommandRun, CommandRunner};
use crate::ports::{BoxFuture, now};

fn json(text: &str) -> JsValue {
    json_parse(text).unwrap()
}

const REQUEST: &str = r#"{"spec": {"form": "shell", "command": "./t.sh", "shell": "bash"},
    "cwd": "/p", "env": {}, "signal": {"$signal": 3, "aborted": false}}"#;

/// A runner that answers `result` and keeps the directory of each run it was asked for.
struct Runner {
    result: CommandResult,
    asked: Mutex<Vec<String>>,
}

impl CommandRunner for Runner {
    fn run(&self, run: CommandRun) -> BoxFuture<'_, CommandResult> {
        self.asked.lock().unwrap().push(run.cwd);
        Box::pin(ready(self.result.clone()))
    }
}

fn runner(result: &str) -> Runner {
    Runner {
        result: CommandResult::deserialize(json(result)).unwrap(),
        asked: Mutex::new(Vec::new()),
    }
}

fn request(text: &str) -> CommandRun {
    CommandRun::deserialize(json(text)).unwrap()
}

/// The run's result once the runner answers `result`.
fn settled(wakes: bool, result: &str) -> BackgroundResult {
    let mode = Mode {
        background: true,
        wakes,
    };
    now(start_in_background(runner(result), request(REQUEST), mode).done).unwrap()
}

const EXIT_2: &str = r#"{"exitCode": 2, "stdout": "out", "stderr": " ", "timedOut": false}"#;

fn exited(exit_code: i32, stdout: &str, stderr: &str) -> ForegroundRun {
    ForegroundRun::Exited(ExitedRun {
        exit_code,
        stdout: stdout.to_owned(),
        stderr: stderr.to_owned(),
        output_held_open: None,
    })
}

#[test]
fn a_rewake_run_that_exits_2_wakes_claude_with_stdout_when_stderr_is_blank() {
    let result = settled(true, EXIT_2);
    assert_eq!(result.run, exited(2, "out", " "));
    assert_eq!(result.wake.as_deref(), Some("out"));
}

#[test]
fn an_async_run_that_exits_2_does_not_wake() {
    let result = settled(false, EXIT_2);
    assert_eq!(result.run, exited(2, "out", " "));
    assert_eq!(result.wake, None);
}

#[test]
fn a_held_open_capture_is_kept_on_the_run() {
    let held =
        r#"{"exitCode": 0, "stdout": "", "stderr": "", "timedOut": false, "outputHeldOpen": true}"#;
    let ForegroundRun::Exited(run) = settled(false, held).run else {
        panic!("the run did not exit")
    };
    assert_eq!(run.output_held_open, Some(true));
}

#[test]
fn a_signal_aborted_before_the_start_runs_nothing() {
    let aborted = REQUEST.replace(r#""aborted": false"#, r#""aborted": true"#);
    let runner = runner(EXIT_2);
    let run = now(settle(&runner, request(&aborted))).unwrap();
    assert_eq!(run, ForegroundRun::Cancelled);
    assert!(runner.asked.lock().unwrap().is_empty());
}
