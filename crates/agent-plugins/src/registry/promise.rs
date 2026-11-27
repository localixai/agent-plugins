//! Promises as values, at any depth of an argument or an answer (docs/RUST.md, "Effects", item 7).
//! Nothing crosses but numbers, and the host still drives every wait:
//!
//! - The host's promise, `{"$promise": n}`, is a [`HostPromise`]: a future whose first poll asks
//!   the need `then` of value `n`, with no arguments, which the host answers when the promise
//!   settles, its value or, for a rejection, a throw (`catch_host` applies, as to any need).
//!   Given back to the host, it crosses as `{"$promise": n}` again and is the host's own promise.
//! - A future of the core's inside an answer is a [`CorePromise`]: crossing as `{"$promise":
//!   {"task": t}}`, it is kept as task `t`, not polled yet; the host makes it a promise of its
//!   own that drives task `t` with the loop of a call (`resume`, the first with no answers), its
//!   needs answered with the host values of the call that made it, and that settles with the
//!   task's answer. The host cancels the task (`registry::cancel`) when it can no longer drive it.
//!
//! A Rust host awaits a [`CorePromise`] as the future it is, and never makes a [`HostPromise`].

use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Mutex, PoisonError};
use std::task::{Context, Poll};

use serde::de::{DeserializeOwned, Deserializer};
use serde::ser::{Error as _, SerializeMap as _, Serializer};
use serde::{Deserialize, Serialize};

use super::remote::answered;
use super::task;
use super::wire::ok;
use crate::ports::BoxFuture;

/// A promise of the host's, numbered `n` on its side, that settles with a `T`.
pub struct HostPromise<T> {
    id: u32,
    /// The need `then`, asked at the first poll.
    settling: Option<BoxFuture<'static, T>>,
}

impl<T> HostPromise<T> {
    /// The promise the host numbers `id`.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self { id, settling: None }
    }
}

impl<T> Clone for HostPromise<T> {
    /// The same promise of the host's, not awaited yet: each await is a `then` of its own.
    fn clone(&self) -> Self {
        Self::new(self.id)
    }
}

impl<T> fmt::Debug for HostPromise<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("HostPromise")
            .field(&self.id)
            .finish()
    }
}

impl<T> PartialEq for HostPromise<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: DeserializeOwned + Send + 'static> Future for HostPromise<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<T> {
        let this = self.get_mut();
        let id = this.id;
        this.settling
            .get_or_insert_with(|| Box::pin(answered(id, "then", vec![])))
            .as_mut()
            .poll(context)
    }
}

/// A promise as the wire spells it, `{"$promise": n}`.
#[derive(Serialize, Deserialize)]
struct PromiseTag {
    #[serde(rename = "$promise")]
    id: u32,
}

impl<'de, T> Deserialize<'de> for HostPromise<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        PromiseTag::deserialize(deserializer).map(|tag| Self::new(tag.id))
    }
}

impl<T> Serialize for HostPromise<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PromiseTag { id: self.id }.serialize(serializer)
    }
}

/// A future of the core's that the host receives as a promise: its output `T` is what the
/// promise resolves to, and a host's throw it does not catch is what it rejects with.
pub struct CorePromise<T> {
    /// The future, until it crosses (then the task table holds it) or is awaited by a Rust host.
    future: Mutex<Option<BoxFuture<'static, T>>>,
    output: PhantomData<fn() -> T>,
}

impl<T: Serialize + Send + 'static> CorePromise<T> {
    /// `future` as a promise. Make it from an `async` block or an `async fn`'s call, which asks
    /// the host nothing until it is polled, so its needs are its own task's: a port method
    /// already called asked in the task that called it (`registry/asked.rs`).
    pub fn new(future: impl Future<Output = T> + Send + 'static) -> Self {
        Self {
            future: Mutex::new(Some(Box::pin(future))),
            output: PhantomData,
        }
    }
}

impl<T> fmt::Debug for CorePromise<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CorePromise")
    }
}

impl<T> Future for CorePromise<T> {
    type Output = T;

    /// A Rust host's await. One that already crossed to the host never completes here.
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<T> {
        let slot = self
            .get_mut()
            .future
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        match slot.as_mut() {
            Some(future) => future.as_mut().poll(context),
            None => Poll::Pending,
        }
    }
}

impl<T: Serialize + Send + 'static> Serialize for CorePromise<T> {
    /// Keeps the future as a new task, `{"$promise": {"task": t}}`; a promise crosses once.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let taken = self
            .future
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let Some(future) = taken else {
            return Err(S::Error::custom("a core promise crosses to the host once"));
        };
        let number = task::keep(Box::pin(async move { ok(future.await) }));
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("$promise", &Tag { task: number })?;
        map.end()
    }
}

/// The task a core promise is kept as, `{"task": t}`.
#[derive(Serialize)]
struct Tag {
    task: u32,
}

#[cfg(test)]
mod tests;

/// A promise of the core's once handed to the host ([`detach`]): the host's own, which any task
/// may await, or, outside a task (a Rust host awaiting the core itself), still the core's. Read
/// from the wire it is the host's.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Detached<T> {
    /// The host's promise.
    Host(HostPromise<T>),
    /// The core's future, for a Rust host.
    #[serde(skip)]
    Core(CorePromise<T>),
}

impl<T: Serialize + DeserializeOwned + Send + 'static> Detached<T> {
    /// What it settles with.
    pub async fn settled(self) -> T {
        match self {
            Self::Host(promise) => promise.await,
            Self::Core(promise) => promise.await,
        }
    }
}

/// Hands `promise` to the host at once, by the need `$hold` with it as the argument: reading the
/// need, the host makes it its promise, which runs to its first wait at once (item 7's order), and
/// answers with that promise. So the work it starts begins while the task that made it goes on, as
/// an `async` function runs to its first `await` when called, and the task that awaits it later
/// need not be this one. Outside a task it stays the core's future.
pub async fn detach<T: Serialize + DeserializeOwned + Send + 'static>(
    promise: CorePromise<T>,
) -> Detached<T> {
    if task::with_current(|_| ()).is_none() {
        return Detached::Core(promise);
    }
    let value = crate::js::value::to_js_value(&promise).unwrap_or_default();
    Detached::Host(answered::<HostPromise<T>>(0, "$hold", vec![value]).await)
}
