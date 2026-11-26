//! `catch_host`: a caught throw, an uncaught one, nested scopes, a rethrow from a catch, two needs
//! at once where one throws, a throw after the scope was dropped, and a Rust host's pass-through.

use std::future::{Future, ready};

use serde::Deserialize as _;

use super::{Thrown, catch_host};
use crate::js::JsValue;
use crate::js::promise_all_pair;
use crate::ports::files::TextFileReader as _;
use crate::ports::now;
use crate::registry::task::{self, Need, Reply, Step};
use crate::registry::{Answer, Port};

fn files() -> Port {
    Port::deserialize(JsValue::from([("$port", JsValue::from(0.0))])).unwrap()
}

/// Runs `work` as a task; its debug text is the answer.
fn started<T: std::fmt::Debug>(work: impl Future<Output = T> + 'static) -> Step {
    task::start(Box::pin(async move {
        Answer::Result(format!("{:?}", work.await).into())
    }))
}

fn waits(step: Step) -> (u32, Vec<u32>) {
    match step {
        Step::Needs { task, needs } => (task, needs.iter().map(|need: &Need| need.id).collect()),
        other @ (Step::Done(_) | Step::Rethrow { .. }) => panic!("it does not wait: {other:?}"),
    }
}

fn throws(id: u32, message: &str) -> (u32, Reply) {
    let name = Some("Error".to_owned());
    let message = message.to_owned();
    (id, Reply::Throws { message, name })
}

fn text(id: u32, value: &str) -> (u32, Reply) {
    (id, Reply::Result(value.into()))
}

fn done(text: &str) -> Step {
    Step::Done(Answer::Result(text.into()))
}

fn thrown(message: &str, need: u32) -> Thrown {
    let name = Some("Error".to_owned());
    Thrown {
        message: message.to_owned(),
        name,
        need,
    }
}

#[test]
fn a_caught_throw_is_the_scopes_err_and_the_task_goes_on() {
    let (number, asked) = waits(started(async {
        let caught = catch_host(files().read_text("a", None)).await;
        let after = files().read_text("b", None).await;
        (caught, after)
    }));
    assert_eq!(asked, vec![0]);
    let (_, asked) = waits(task::resume(number, vec![throws(0, "EACCES")]));
    assert_eq!(asked, vec![1]);
    let step = task::resume(number, vec![text(1, "B")]);
    let expected = format!("({:?}, Some(\"B\"))", Err::<(), _>(thrown("EACCES", 0)));
    assert_eq!(step, done(&expected));
}

#[test]
fn an_answer_inside_a_scope_is_its_ok() {
    let (number, _) = waits(started(async {
        catch_host(files().read_text("a", None)).await
    }));
    let step = task::resume(number, vec![text(0, "A")]);
    assert_eq!(step, done(r#"Ok(Some("A"))"#));
}

#[test]
fn a_throw_outside_every_scope_ends_the_task() {
    let (number, _) = waits(started(async {
        let first = catch_host(ready(1)).await;
        (first, files().read_text("a", None).await)
    }));
    assert_eq!(
        task::resume(number, vec![throws(0, "EIO")]),
        Step::Rethrow { need: 0 }
    );
}

#[test]
fn the_innermost_scope_catches_and_a_rethrow_reaches_the_outer_one() {
    let (number, _) = waits(started(catch_host(async {
        let inner = catch_host(files().read_text("a", None)).await;
        let again = catch_host(async {
            match catch_host(files().read_text("b", None)).await {
                Ok(text) => text,
                Err(cause) => cause.rethrow().await,
            }
        })
        .await;
        (inner, again)
    })));
    let (_, asked) = waits(task::resume(number, vec![throws(0, "first")]));
    assert_eq!(asked, vec![1]);
    let step = task::resume(number, vec![throws(1, "second")]);
    let inner = Err::<(), _>(thrown("first", 0));
    let again = Err::<(), _>(thrown("second", 1));
    assert_eq!(step, done(&format!("Ok(({inner:?}, {again:?}))")));
}

#[test]
fn a_rethrow_outside_every_scope_ends_the_task_with_the_need() {
    let (number, _) = waits(started(async {
        match catch_host(files().read_text("a", None)).await {
            Ok(text) => text,
            Err(cause) => cause.rethrow().await,
        }
    }));
    let step = task::resume(number, vec![throws(0, "aborted")]);
    assert_eq!(step, Step::Rethrow { need: 0 });
    assert!(!task::cancel(number));
}

#[test]
fn two_needs_at_once_one_answers_first_and_the_other_throws() {
    let (number, asked) = waits(started(async {
        catch_host(promise_all_pair(
            files().read_text("a", None),
            files().read_text("b", None),
        ))
        .await
    }));
    assert_eq!(asked, vec![0, 1]);
    let (_, none) = waits(task::resume(number, vec![text(1, "B")]));
    assert!(none.is_empty());
    let step = task::resume(number, vec![throws(0, "EACCES")]);
    assert_eq!(
        step,
        done(&format!("{:?}", Err::<(), _>(thrown("EACCES", 0))))
    );
}

#[test]
fn a_throw_drops_the_other_need_whose_late_answer_or_throw_is_ignored() {
    let (number, asked) = waits(started(async {
        let both = catch_host(promise_all_pair(
            files().read_text("a", None),
            files().read_text("b", None),
        ))
        .await;
        (both.is_err(), files().read_text("c", None).await)
    }));
    assert_eq!(asked, vec![0, 1]);
    let (_, asked) = waits(task::resume(number, vec![throws(0, "EACCES")]));
    assert_eq!(asked, vec![2]);
    // Need 1's future is gone: its throw neither ends the task nor reaches a scope.
    let (_, none) = waits(task::resume(number, vec![throws(1, "late")]));
    assert!(none.is_empty());
    assert_eq!(
        task::resume(number, vec![text(2, "C")]),
        done(r#"(true, Some("C"))"#)
    );
}

#[test]
fn a_need_asked_in_the_poll_that_caught_is_never_handed_to_the_host() {
    let (number, asked) = waits(started(async {
        let second = Box::pin(async {
            files().read_text("b", None).await;
            files().read_text("c", None).await
        });
        let caught = catch_host(promise_all_pair(files().read_text("a", None), second)).await;
        (caught.is_err(), files().read_text("d", None).await)
    }));
    assert_eq!(asked, vec![0, 1]);
    // Need 1's answer makes the second branch ask need 2 in the poll where need 0 throws.
    let (_, asked) = waits(task::resume(number, vec![text(1, "B"), throws(0, "EIO")]));
    assert_eq!(asked, vec![3]);
    let step = task::resume(number, vec![text(3, "D")]);
    assert_eq!(step, done(r#"(true, Some("D"))"#));
}

#[test]
fn a_need_awaited_after_its_scope_ended_is_uncaught() {
    let (number, _) = waits(started(async {
        let port = files();
        let later = catch_host(ready(port.read_text("a", None))).await;
        match later {
            Ok(future) => future.await,
            Err(_) => None,
        }
    }));
    let step = task::resume(number, vec![throws(0, "EIO")]);
    assert_eq!(step, Step::Rethrow { need: 0 });
}

#[test]
fn a_rust_host_sees_a_pass_through() {
    assert_eq!(now(catch_host(ready(5))), Some(Ok(5)));
}

/// A scope and a rethrow can run inside `promise_all`, whose futures are `Send`.
#[test]
fn a_scope_and_a_rethrow_are_send() {
    fn send<T: Send>(_: T) {}
    send(catch_host(ready(1)));
    send(thrown("x", 0).rethrow::<u8>());
}
