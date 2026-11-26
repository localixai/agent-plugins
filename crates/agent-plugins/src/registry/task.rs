//! The tasks a host drives: an export that waits on the host (a port, a callback) is a future the
//! registry keeps between [`start`](crate::registry::start) and
//! [`resume`](crate::registry::resume), polled with a waker that does nothing, because the host,
//! not a waker, says when to poll again: when it answers.
//!
//! A port method of the host's (`registry::Port`) does not call out: it writes a [`Need`] into
//! the link of the task being polled and returns a future that is pending until the host's
//! answer is in that link. Every need written during one poll is handed to the host together, so
//! a `join` in the core is concurrent work on the host, as `Promise.all` is in the TypeScript.
//!
//! The tables are the thread's: the WebAssembly module has one thread, and a Rust host that
//! drives tasks on several threads gets a table on each.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use super::Answer;
use crate::js::JsValue;

/// An export's work as the registry keeps it: a future that gives the export's answer once the
/// host has answered what it waits on.
pub type Started = Pin<Box<dyn Future<Output = Answer>>>;

/// One thing the core asks of the host: `method` of the host value numbered `port` (a port, or
/// a callback, whose method is `call`) with `args`. `id` is the need's number within its task.
#[derive(Debug, Clone, PartialEq)]
pub struct Need {
    /// The need's number within its task, which the host's answer names.
    pub id: u32,
    /// The number the host gave the value (`{"$port": n}`, `{"$fn": n}`).
    pub port: u32,
    /// The method's name.
    pub method: String,
    /// The arguments.
    pub args: Vec<JsValue>,
}

/// The host's answer to a need.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// What the method returned (or its promise resolved to).
    Result(JsValue),
    /// The method threw (or its promise rejected), with the message of what it threw.
    Throws(String),
}

/// Where a task is after a poll.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// The export answered; the task is gone.
    Done(Answer),
    /// The task waits: on `needs`, asked now, and on any asked before and not answered yet.
    Needs {
        /// The task's number, for `resume`.
        task: u32,
        /// What the host is asked now.
        needs: Vec<Need>,
    },
    /// The host threw for `need`, so the export throws what the host threw (its own error, which
    /// only the host holds); the task is gone.
    Rethrow {
        /// The need the host threw for.
        need: u32,
    },
}

/// Why a task ended before its future did.
#[derive(Debug)]
enum End {
    Rethrow(u32),
    Fault(String),
}

/// What a task shares with the port calls it makes while it is polled.
#[derive(Debug, Default)]
struct Link {
    next_need: u32,
    asked: Vec<Need>,
    waiting: BTreeSet<u32>,
    replies: BTreeMap<u32, Reply>,
    end: Option<End>,
}

struct Task {
    future: Started,
    link: Link,
}

#[derive(Default)]
struct Tasks {
    next: u32,
    running: BTreeMap<u32, Task>,
}

thread_local! {
    static TASKS: Mutex<Tasks> = Mutex::new(Tasks::default());
    /// The link of the task being polled, moved here for the poll.
    static CURRENT: Mutex<Option<Link>> = const { Mutex::new(None) };
}

/// A lock that a panic elsewhere cannot poison: the tables hold no invariant a panic breaks.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The link of the task being polled, given to `body`; `None` outside a poll.
fn with_current<R>(body: impl FnOnce(&mut Link) -> R) -> Option<R> {
    CURRENT.with(|current| lock(current).as_mut().map(body))
}

/// Asks the host for `method` of value `port`; the future is the host's answer.
pub(crate) fn ask(port: u32, method: &str, args: Vec<JsValue>) -> Asked {
    let id = with_current(|link| {
        let id = link.next_need;
        link.next_need += 1;
        link.waiting.insert(id);
        link.asked.push(Need {
            id,
            port,
            method: method.to_owned(),
            args,
        });
        id
    });
    Asked { id }
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
    /// `None` when it was asked outside a task, where no host will ever answer.
    id: Option<u32>,
}

impl Future for Asked {
    type Output = JsValue;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<JsValue> {
        let Some(id) = self.id else {
            fail("a host value was used outside a task the host drives".to_owned());
            return Poll::Pending;
        };
        match with_current(|link| link.replies.remove(&id)).flatten() {
            Some(Reply::Result(value)) => Poll::Ready(value),
            Some(Reply::Throws(_)) => {
                with_current(|link| {
                    link.end.get_or_insert(End::Rethrow(id));
                });
                Poll::Pending
            }
            None => Poll::Pending,
        }
    }
}

/// Polls a task once and says where it is; a task that still waits goes back into the table.
fn drive(number: u32, mut task: Task) -> Step {
    CURRENT.with(|current| *lock(current) = Some(mem::take(&mut task.link)));
    let polled = task
        .future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    task.link = CURRENT
        .with(|current| lock(current).take())
        .unwrap_or_default();
    if let Poll::Ready(answer) = polled {
        return Step::Done(answer);
    }
    match task.link.end.take() {
        Some(End::Rethrow(need)) => return Step::Rethrow { need },
        Some(End::Fault(why)) => return Step::Done(Answer::Fault(why)),
        None => {}
    }
    if task.link.waiting.is_empty() {
        return Step::Done(Answer::Fault(
            "the export waits on nothing the host was asked".to_owned(),
        ));
    }
    let needs = mem::take(&mut task.link.asked);
    TASKS.with(|tasks| lock(tasks).running.insert(number, task));
    Step::Needs {
        task: number,
        needs,
    }
}

/// Keeps an export's future as a new task and polls it the first time.
pub(crate) fn start(future: Started) -> Step {
    let number = TASKS.with(|tasks| {
        let mut tasks = lock(tasks);
        let number = tasks.next;
        tasks.next = tasks.next.wrapping_add(1);
        number
    });
    drive(
        number,
        Task {
            future,
            link: Link::default(),
        },
    )
}

/// Gives a waiting task the host's answers and polls it again. An answer to a need the task does
/// not wait on is a fault of the host's loop, which ends the task.
pub(crate) fn resume(number: u32, replies: Vec<(u32, Reply)>) -> Step {
    let Some(mut task) = TASKS.with(|tasks| lock(tasks).running.remove(&number)) else {
        return Step::Done(Answer::Fault(format!(
            "no task {number}: it ended or never started"
        )));
    };
    for (id, reply) in replies {
        if !task.link.waiting.remove(&id) {
            return Step::Done(Answer::Fault(format!(
                "task {number} does not wait on need {id}"
            )));
        }
        task.link.replies.insert(id, reply);
    }
    drive(number, task)
}

/// Drops a waiting task, its future and what it waits on, as a host that gives up on it does;
/// whether there was one.
pub fn cancel(number: u32) -> bool {
    TASKS.with(|tasks| lock(tasks).running.remove(&number).is_some())
}

/// How many tasks wait, for the tests.
#[cfg(test)]
pub(crate) fn waiting() -> usize {
    TASKS.with(|tasks| lock(tasks).running.len())
}
