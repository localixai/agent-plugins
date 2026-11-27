//! A need's future: what a port method of the host's (`registry::Port`) returns. [`ask`] writes
//! the need into the link of the task being polled, at once, as calling a port method in
//! JavaScript starts it; the future is pending until the host's answer is in that link.
//!
//! A need belongs to the task that asked it: awaited while another task is polled (a
//! `registry::CorePromise` made from a port call already started, rather than from an `async`
//! block that starts it), it ends that task with a fault, as no answer of the other task's can
//! reach it.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::catch::{self, Thrown};
use super::task::{End, Need, Reply, with_current};
use crate::js::JsValue;

/// Asks the host for `method` of value `port`; the future is the host's answer.
pub(crate) fn ask(port: u32, method: &str, args: Vec<JsValue>) -> Asked {
    let asked = with_current(|link| {
        let id = link.next_need;
        link.next_need += 1;
        link.waiting.insert(id);
        link.asked.push(Need {
            id,
            port,
            method: method.to_owned(),
            args,
        });
        (link.number, id)
    });
    Asked {
        asked,
        settled: false,
    }
}

/// Ends the task being polled with a fault: the host answered what the port's type does not hold.
pub(crate) fn fail(why: String) {
    with_current(|link| {
        link.end.get_or_insert(End::Fault(why));
    });
}

/// A need's answer: pending until the host gives it.
#[derive(Debug)]
pub(crate) struct Asked {
    /// The task and the need's number in it; `None` when it was asked outside a task, where no
    /// host will ever answer.
    asked: Option<(u32, u32)>,
    /// Whether its answer was read.
    settled: bool,
}

/// Whether the task being polled is `task`.
fn polling(task: u32) -> bool {
    with_current(|link| link.number == task).unwrap_or(false)
}

impl Future for Asked {
    type Output = JsValue;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<JsValue> {
        let Some((task, id)) = self.asked else {
            fail("a host value was used outside a task the host drives".to_owned());
            return Poll::Pending;
        };
        if !polling(task) {
            fail(format!(
                "need {id} of task {task} is awaited in another task: make a core promise from an \
                 async block, which asks in its own task"
            ));
            return Poll::Pending;
        }
        let reply = with_current(|link| link.replies.remove(&id)).flatten();
        self.settled |= reply.is_some();
        match reply {
            Some(Reply::Result(value)) => Poll::Ready(value),
            Some(Reply::Throws { message, name }) => {
                catch::raise(Thrown {
                    message,
                    name,
                    need: id,
                });
                Poll::Pending
            }
            None => Poll::Pending,
        }
    }
}

impl Drop for Asked {
    fn drop(&mut self) {
        if let (Some((task, id)), false) = (self.asked, self.settled)
            && polling(task)
        {
            with_current(|link| link.retract(id));
        }
    }
}
