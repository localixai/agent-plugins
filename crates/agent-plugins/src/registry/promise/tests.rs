//! Promises as values: the host's awaited through `then`, a rejection caught and uncaught, a
//! round trip, a core promise kept as a task and driven, dropped before it settles, two
//! interleaved, one made from a port call already started, one that crosses twice, and a Rust
//! host's await.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize as _, Serialize};

use super::{CorePromise, HostPromise};
use crate::js::JsValue;
use crate::js::json_parse::json_parse;
use crate::ports::now;
use crate::registry::task::{self, Need, Reply, Step};
use crate::registry::wire::ok;
use crate::registry::{Answer, Callback, catch_host};

fn json(text: &str) -> JsValue {
    json_parse(text).unwrap()
}

fn promise(id: u32) -> HostPromise<String> {
    HostPromise::deserialize(json(&format!(r#"{{"$promise": {id}}}"#))).unwrap()
}

fn callback(id: u32) -> Callback {
    Callback::deserialize(json(&format!(r#"{{"$fn": {id}}}"#))).unwrap()
}

fn waits(step: Step) -> (u32, Vec<Need>) {
    match step {
        Step::Needs { task, needs } => (task, needs),
        other @ (Step::Done(_) | Step::Rethrow { .. }) => panic!("it does not wait: {other:?}"),
    }
}

fn result(id: u32, value: &str) -> (u32, Reply) {
    (id, Reply::Result(value.into()))
}

fn rejected(id: u32, message: &str) -> (u32, Reply) {
    let name = Some("Error".to_owned());
    (
        id,
        Reply::Throws {
            message: message.to_owned(),
            name,
        },
    )
}

fn done(text: &str) -> Step {
    Step::Done(Answer::Result(text.into()))
}

#[test]
fn a_host_promise_is_awaited_by_the_need_then_with_no_arguments() {
    let (number, needs) = waits(task::start(Box::pin(async { ok(promise(4).await) })));
    let then = Need {
        id: 0,
        port: 4,
        method: "then".to_owned(),
        args: vec![],
    };
    assert_eq!(needs, vec![then]);
    assert_eq!(
        task::resume(number, vec![result(0, "settled")]),
        done("settled")
    );
}

#[test]
fn a_rejection_inside_catch_host_is_its_err_and_outside_ends_the_task() {
    let (number, _) = waits(task::start(Box::pin(async {
        match catch_host(promise(4)).await {
            Ok(text) => ok(text),
            Err(cause) => ok(format!("caught {} of need {}", cause.message, cause.need)),
        }
    })));
    let step = task::resume(number, vec![rejected(0, "process vanished")]);
    assert_eq!(step, done("caught process vanished of need 0"));

    let (number, _) = waits(task::start(Box::pin(async { ok(promise(4).await) })));
    let step = task::resume(number, vec![rejected(0, "process vanished")]);
    assert_eq!(step, Step::Rethrow { need: 0 });
}

#[test]
fn a_host_promise_given_back_is_the_hosts_own_number_again() {
    let value = crate::js::value::to_js_value(&vec![promise(4), promise(9)]).unwrap();
    assert_eq!(value, json(r#"[{"$promise": 4}, {"$promise": 9}]"#));
}

/// An answer that holds `promise` under `done`, as `startInBackground`'s does.
#[derive(Serialize)]
struct Holder {
    done: CorePromise<String>,
}

/// The task a core promise crossed as: `make` runs inside a call's task, whose answer
/// `{"done": {"$promise": {"task": t}}}` holds what it made.
fn crossed(make: impl FnOnce() -> CorePromise<String> + 'static) -> u32 {
    let Step::Done(Answer::Result(value)) =
        task::start(Box::pin(async move { ok(Holder { done: make() }) }))
    else {
        panic!("the answer did not encode")
    };
    let tag = value.get("done").and_then(|done| done.get("$promise"));
    let number = tag
        .and_then(|tag| tag.get("task"))
        .and_then(JsValue::as_f64);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a task number"
    )]
    number
        .map(|number| number as u32)
        .unwrap_or_else(|| panic!("no core promise in {value:?}"))
}

#[test]
fn a_core_promise_is_a_task_polled_first_by_a_resume_with_no_answers() {
    let started = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&started);
    let number = crossed(move || {
        CorePromise::new(async move {
            seen.store(true, Ordering::SeqCst);
            callback(2).call::<String>(vec!["go".into()]).await
        })
    });
    assert!(!started.load(Ordering::SeqCst), "kept, not polled");
    let (again, needs) = waits(task::resume(number, vec![]));
    assert_eq!(again, number);
    let call = Need {
        id: 0,
        port: 2,
        method: "call".to_owned(),
        args: vec!["go".into()],
    };
    assert_eq!(needs, vec![call]);
    assert_eq!(task::resume(number, vec![result(0, "went")]), done("went"));
}

#[test]
fn a_core_promise_dropped_before_it_settles_drops_its_future_and_its_late_answer() {
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let guard = Guard(Arc::clone(&dropped));
    let number = crossed(move || {
        CorePromise::new(async move {
            let _held = guard;
            callback(2).call::<String>(vec![]).await
        })
    });
    let before = task::waiting();
    let _ = waits(task::resume(number, vec![]));
    assert!(task::cancel(number));
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(task::waiting(), before - 1);
    let late = task::resume(number, vec![result(0, "late")]);
    assert!(matches!(late, Step::Done(Answer::Fault(why)) if why.contains("no task")));
}

#[test]
fn two_core_promises_run_apart_each_with_its_own_needs() {
    let first = crossed(move || {
        CorePromise::new(async {
            let a = callback(1).call::<String>(vec![]).await;
            let b = callback(1).call::<String>(vec![]).await;
            format!("{a}{b}")
        })
    });
    let second =
        crossed(move || CorePromise::new(async { callback(2).call::<String>(vec![]).await }));
    let (_, one) = waits(task::resume(first, vec![]));
    let (_, two) = waits(task::resume(second, vec![]));
    assert_eq!(
        (one[0].id, two[0].id),
        (0, 0),
        "a need's number is its task's"
    );
    let (_, one) = waits(task::resume(first, vec![result(0, "a")]));
    assert_eq!(task::resume(second, vec![result(0, "x")]), done("x"));
    assert_eq!(one[0].id, 1);
    assert_eq!(task::resume(first, vec![result(1, "b")]), done("ab"));
}

#[test]
fn a_core_promise_made_from_a_port_call_already_started_is_a_fault() {
    let inner = crossed(|| CorePromise::new(callback(1).call::<String>(vec![])));
    let step = task::resume(inner, vec![]);
    assert!(matches!(step, Step::Done(Answer::Fault(why)) if why.contains("another task")));
}

#[test]
fn a_core_promise_crosses_once() {
    let promise = CorePromise::new(async { "once".to_owned() });
    let first = crate::js::value::to_js_value(&promise);
    let second = crate::js::value::to_js_value(&promise);
    assert!(first.is_ok());
    assert!(second.is_err_and(|error| error.to_string().contains("once")));
    task::cancel(crossed(move || CorePromise::new(std::future::pending())));
}

#[test]
fn a_rust_host_awaits_a_core_promise_as_its_future() {
    let promise = CorePromise::new(async { "here".to_owned() });
    assert_eq!(now(promise), Some("here".to_owned()));
}
