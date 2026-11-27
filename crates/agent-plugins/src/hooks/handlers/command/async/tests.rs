//! A background run through the registry: the answer holds a promise of the core's, kept as a
//! task, whose first resume asks the runner; its result wakes Claude on exit 2 for `rewake` only.
//! And `settle`: a signal aborted before the start runs nothing.

use crate::js::JsValue;
use crate::js::json_parse::json_parse;
use crate::registry::{Answer, Reply, Step, resume, start};

fn json(text: &str) -> JsValue {
    json_parse(text).unwrap()
}

const REQUEST: &str = r#"{"spec": {"form": "shell", "command": "./t.sh", "shell": "bash"},
    "cwd": "/p", "env": {}, "signal": {"$signal": 3, "aborted": false}}"#;

/// The background run's task for `mode`, from the answer `{"kind": "background", "done": …}`.
fn background(mode: &str) -> u32 {
    let args = [json(r#"{"$port": 1}"#), json(REQUEST), json(mode)];
    let Step::Done(Answer::Result(value)) =
        start("hooks/handlers/command/async#startInBackground", &args)
    else {
        panic!("it does not answer at once")
    };
    assert_eq!(
        value.get("kind").and_then(JsValue::as_str),
        Some("background")
    );
    let task = value
        .get("done")
        .and_then(|done| done.get("$promise"))
        .and_then(|tag| tag.get("task"));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a task number"
    )]
    task.and_then(JsValue::as_f64)
        .map(|task| task as u32)
        .unwrap()
}

/// The run's result once the runner answers `result`.
fn settled(mode: &str, result: &str) -> Step {
    let task = background(mode);
    let Step::Needs { needs, .. } = resume(task, vec![], &[]) else {
        panic!("the run does not ask the runner")
    };
    assert_eq!(needs.len(), 1);
    assert_eq!((needs[0].port, needs[0].method.as_str()), (1, "run"));
    assert_eq!(needs[0].args.first(), Some(&json(REQUEST)));
    resume(task, vec![(needs[0].id, Reply::Result(json(result)))], &[])
}

const EXIT_2: &str = r#"{"exitCode": 2, "stdout": "out", "stderr": " ", "timedOut": false}"#;

#[test]
fn a_rewake_run_that_exits_2_wakes_claude_with_stdout_when_stderr_is_blank() {
    let wake = json(
        r#"{"run": {"kind": "exited", "exitCode": 2, "stdout": "out", "stderr": " "}, "wake": "out"}"#,
    );
    assert_eq!(
        settled(r#"{"background": true, "wakes": true}"#, EXIT_2),
        Step::Done(Answer::Result(wake))
    );
}

#[test]
fn an_async_run_that_exits_2_does_not_wake() {
    let run = json(r#"{"run": {"kind": "exited", "exitCode": 2, "stdout": "out", "stderr": " "}}"#);
    assert_eq!(
        settled(r#"{"background": true, "wakes": false}"#, EXIT_2),
        Step::Done(Answer::Result(run))
    );
}

#[test]
fn a_held_open_capture_is_kept_on_the_run() {
    let held =
        r#"{"exitCode": 0, "stdout": "", "stderr": "", "timedOut": false, "outputHeldOpen": true}"#;
    let run = json(
        r#"{"run": {"kind": "exited", "exitCode": 0, "stdout": "", "stderr": "", "outputHeldOpen": true}}"#,
    );
    assert_eq!(
        settled(r#"{"background": true, "wakes": false}"#, held),
        Step::Done(Answer::Result(run))
    );
}

#[test]
fn a_signal_aborted_before_the_start_runs_nothing() {
    let aborted = REQUEST.replace(r#""aborted": false"#, r#""aborted": true"#);
    let args = [json(r#"{"$port": 1}"#), json(&aborted)];
    let step = start("hooks/handlers/command/async#settle", &args);
    assert_eq!(
        step,
        Step::Done(Answer::Result(json(r#"{"kind": "cancelled"}"#)))
    );
}
