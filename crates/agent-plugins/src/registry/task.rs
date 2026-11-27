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
//! A host's throw ends the task (`{"rethrow": id}`) unless a [`catch_host`](crate::registry::catch_host) scope awaits the
//! need: then the scope answers it (`registry/catch.rs`).
//!
//! A task is an export's call, a handle method's call, or a future of the core's that crossed in
//! an answer as a promise (`registry::CorePromise`): that one is kept without a poll ([`keep`]),
//! and the host's first `resume` of it, with no answers, polls it the first time.
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
pub(crate) use super::asked::{ask, fail};
use super::catch::Thrown;
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
    /// The method threw (or its promise rejected): the message of what it threw (`cause.message`
    /// of an `Error`, `String(cause)` of anything else) and an `Error`'s `name`.
    Throws {
        /// `cause instanceof Error ? cause.message : String(cause)`.
        message: String,
        /// The error's `name` (`TypeError`), when it is an `Error`.
        name: Option<String>,
    },
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
pub(super) enum End {
    Rethrow(u32),
    Fault(String),
}

/// What a task shares with the port calls it makes while it is polled.
#[derive(Debug, Default)]
pub(super) struct Link {
    /// The task's number: a need belongs to the task that asked it (`registry/asked.rs`).
    pub(super) number: u32,
    pub(super) next_need: u32,
    pub(super) asked: Vec<Need>,
    pub(super) waiting: BTreeSet<u32>,
    /// Needs the host was asked whose future is gone (a scope dropped it): their answers, when
    /// they come, are dropped too.
    abandoned: BTreeSet<u32>,
    pub(super) replies: BTreeMap<u32, Reply>,
    pub(super) end: Option<End>,
    /// The `catch_host` scopes being polled, innermost last, each with the throw it caught.
    pub(super) scopes: Vec<Option<Thrown>>,
}

impl Link {
    /// The host threw `thrown`: the innermost scope being polled catches it, or the task ends.
    pub(super) fn raise(&mut self, thrown: Thrown) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.get_or_insert(thrown);
        } else {
            self.end.get_or_insert(End::Rethrow(thrown.need));
        }
    }

    /// Need `id`'s future is gone before its answer: a need not handed to the host yet is never
    /// handed, one the host is doing is abandoned, an answer not read yet is dropped.
    pub(super) fn retract(&mut self, id: u32) {
        let before = self.asked.len();
        self.asked.retain(|need| need.id != id);
        if self.asked.len() < before {
            self.waiting.remove(&id);
        } else if self.waiting.contains(&id) {
            self.abandoned.insert(id);
        } else {
            self.replies.remove(&id);
        }
    }
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
pub(super) fn with_current<R>(body: impl FnOnce(&mut Link) -> R) -> Option<R> {
    CURRENT.with(|current| lock(current).as_mut().map(body))
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
    if task.link.waiting.is_subset(&task.link.abandoned) {
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

/// Keeps `future` as a new task without polling it; its number. The first `resume` (with no
/// answers) polls it.
pub(crate) fn keep(future: Started) -> u32 {
    TASKS.with(|tasks| {
        let mut tasks = lock(tasks);
        let number = tasks.next;
        tasks.next = tasks.next.wrapping_add(1);
        let link = Link {
            number,
            ..Link::default()
        };
        tasks.running.insert(number, Task { future, link });
        number
    })
}

/// Keeps an export's future as a new task and polls it the first time.
pub(crate) fn start(future: Started) -> Step {
    resume(keep(future), vec![])
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
        if !task.link.abandoned.remove(&id) {
            task.link.replies.insert(id, reply);
        }
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
