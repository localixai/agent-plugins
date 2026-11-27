//! The host's values inside the core, for an export a host drives (`registry::start`): what
//! cannot be copied as data crosses as a number the host keeps it under, and each use of it is a
//! need (`registry::task`).
//!
//! - A port, `{"$port": n}`, is a [`Port`]: it implements every port trait (`crate::ports`), one
//!   file of `remote/` each, each method a need named as the TypeScript port's method, with the
//!   same arguments (`files.readText(path, signal)` is the need `readText` with `[path, signal]`),
//!   its answer read as the Rust type. A property of a port (`ScanFolder.dir`) is read by the
//!   need `.dir`, with no arguments, answered with its value ([`Port::member`]). A port's own
//!   failure (`T | PortFailure` in the TypeScript) is an `Err`; what the host throws ends the
//!   task (`{"rethrow": id}`), unless a `registry::catch_host` scope awaits it.
//! - A callback, `{"$fn": n}`, is a [`Callback`]: calling it is the need `call`.
//! - An `AbortSignal`, `{"$signal": n, "aborted": bool}`, is a [`Signal`] whose flag the host
//!   sets when it resumes a task (`"aborted": [n]`); passed to a port, it crosses back as
//!   `{"$signal": n, "aborted": bool}` and is the host's own signal again. A signal the core made
//!   itself crosses as `{"$signal": null, "aborted": bool}`, a new one on the host's side. The
//!   core never waits on an abort: whatever waits is the host's, and the host's port watches its
//!   own signal.

mod clock;
mod command_runner;
mod environment;
mod files;
mod folder_scan;
mod http_client;
mod locations;
mod marketplace;
mod mcp_bundle;
mod mcp_caller;
mod model_runner;
mod path_resolver;
mod scratch_files;

use std::collections::BTreeMap;
use std::future::{Future, pending};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use serde::de::{DeserializeOwned, Deserializer};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use super::task;
use crate::js::JsValue;
use crate::ports::{BoxFuture, PortFailure, Signal};

/// The host's answer to `method` of value `id`, read as `R`. An answer `R` cannot hold ends the
/// task with a fault: the host broke the port's contract.
pub(super) fn answered<R: DeserializeOwned + Send + 'static>(
    id: u32,
    method: &str,
    args: Vec<JsValue>,
) -> impl Future<Output = R> + Send + use<R> {
    let asked = task::ask(id, method, args);
    let method = method.to_owned();
    async move {
        match R::deserialize(asked.await) {
            Ok(value) => value,
            Err(error) => {
                task::fail(format!(
                    "the host's {method} answered what its type does not hold: {error}"
                ));
                pending().await
            }
        }
    }
}

/// A port the host implements, numbered `n` on its side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    #[serde(rename = "$port")]
    id: u32,
}

/// A function of the host's, numbered `n` on its side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Callback {
    #[serde(rename = "$fn")]
    id: u32,
}

impl Callback {
    /// Calls it with `args`; the future is what it returned, read as `R`.
    pub fn call<R: DeserializeOwned + Send + 'static>(
        self,
        args: Vec<JsValue>,
    ) -> impl Future<Output = R> + Send + 'static {
        answered(self.id, "call", args)
    }
}

thread_local! {
    /// The flags of the host's signals by their number, while a [`Signal`] holds one.
    static SIGNALS: Mutex<BTreeMap<u32, Weak<AtomicBool>>> = const { Mutex::new(BTreeMap::new()) };
}

/// The host aborted its signal `id`: every [`Signal`] made from it is aborted.
pub(crate) fn aborted(id: u32) {
    SIGNALS.with(|signals| {
        let signals = signals.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(flag) = signals.get(&id).and_then(Weak::upgrade) {
            flag.store(true, Ordering::SeqCst);
        }
    });
}

/// The flag of the host's signal `id`, the one already held or a new one.
fn flag_of(id: u32, aborted: bool) -> Arc<AtomicBool> {
    SIGNALS.with(|signals| {
        let mut signals = signals.lock().unwrap_or_else(PoisonError::into_inner);
        signals.retain(|_, flag| flag.strong_count() > 0);
        let flag = signals
            .get(&id)
            .and_then(Weak::upgrade)
            .unwrap_or_else(|| Arc::new(AtomicBool::new(aborted)));
        if aborted {
            flag.store(true, Ordering::SeqCst);
        }
        signals.insert(id, Arc::downgrade(&flag));
        flag
    })
}

/// A signal as the wire spells it.
#[derive(Deserialize)]
struct SignalTag {
    #[serde(rename = "$signal")]
    id: u32,
    aborted: bool,
}

impl<'de> Deserialize<'de> for Signal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let SignalTag { id, aborted } = SignalTag::deserialize(deserializer)?;
        Ok(Self::from_host(id, flag_of(id, aborted)))
    }
}

impl Serialize for Signal {
    /// `{"$signal": n, "aborted": bool}`, `n` the host's number or `null` for a signal the core
    /// made itself.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Tag {
            #[serde(rename = "$signal")]
            id: Option<u32>,
            aborted: bool,
        }
        Tag {
            id: self.host(),
            aborted: self.is_aborted(),
        }
        .serialize(serializer)
    }
}

/// An argument of a need: a value, or `undefined` for `None` (a signal not given).
fn arg<T: Serialize>(value: Option<&T>) -> JsValue {
    value
        .and_then(|value| crate::js::value::to_js_value(value).ok())
        .unwrap_or_default()
}

/// A port's answer that is its value or its failure (`T | PortFailure` in the TypeScript).
#[derive(Deserialize)]
#[serde(untagged)]
enum Failable<T> {
    Failure(PortFailure),
    Value(T),
}

impl<T> Failable<T> {
    fn into_result(self) -> Result<T, PortFailure> {
        match self {
            Self::Failure(failure) => Err(failure),
            Self::Value(value) => Ok(value),
        }
    }
}

impl Port {
    /// The member `name` of the host's object, read as `R` (the need `.<name>`): a property of a
    /// port (`ScanFolder.dir`), or a member of a record that crossed as a port because it holds a
    /// function (docs/RUST.md, "Effects"), a function read as a [`Callback`]; an absent member is
    /// `undefined`.
    pub fn member<R: DeserializeOwned + Send + 'static>(
        self,
        name: &str,
    ) -> impl Future<Output = R> + Send + use<R> {
        answered(self.id, &format!(".{name}"), vec![])
    }

    /// `name(...args)` of the host's object called as its method (`this` is the object), the
    /// answer read as `R`: for an object the TypeScript types as an interface of its own module
    /// rather than a port of `crate::ports` (a policy, a catalog, a token counter), whose module
    /// implements its trait for `Port` with this.
    pub fn method<R: DeserializeOwned + Send + 'static>(
        self,
        name: &str,
        args: Vec<JsValue>,
    ) -> impl Future<Output = R> + Send + use<R> {
        answered(self.id, name, args)
    }

    /// `method(...args)` of the host's port, its answer read as `R`.
    fn ask<R: DeserializeOwned + Send + 'static>(
        self,
        method: &'static str,
        args: Vec<JsValue>,
    ) -> BoxFuture<'static, R> {
        Box::pin(answered(self.id, method, args))
    }

    /// `method(...args)` answered `T | PortFailure`, read as a `Result`.
    fn ask_failable<T: DeserializeOwned + Send + 'static>(
        self,
        method: &'static str,
        args: Vec<JsValue>,
    ) -> BoxFuture<'static, Result<T, PortFailure>> {
        let asked = self.ask::<Failable<T>>(method, args);
        Box::pin(async move { asked.await.into_result() })
    }

    /// `method(path, signal)` of the host's port, read as `R`.
    fn at<R: DeserializeOwned + Send + 'static>(
        self,
        method: &'static str,
        path: &str,
        signal: Option<&Signal>,
    ) -> BoxFuture<'static, R> {
        self.ask(method, vec![JsValue::from(path), arg(signal)])
    }
}

#[cfg(test)]
pub(super) mod testing {
    //! A need asked of a port and its answer read, for the tests of `remote/`.

    use std::fmt::Debug;
    use std::future::Future;

    use serde::Deserialize as _;

    use super::Port;
    use crate::js::JsValue;
    use crate::registry::Answer;
    use crate::registry::task::{self, Need, Reply, Step};

    /// The port the host numbers 7.
    pub(crate) fn port() -> Port {
        Port::deserialize(JsValue::from([("$port", JsValue::from(7.0))])).unwrap()
    }

    /// Runs `work` as a task the host drives: the one need it asks, and what it made of
    /// `answer`, as its debug text.
    pub(crate) fn exchange<T: Debug + 'static>(
        work: impl Future<Output = T> + 'static,
        answer: JsValue,
    ) -> (Need, String) {
        let Step::Needs { task, needs } = task::start(Box::pin(async move {
            Answer::Result(format!("{:?}", work.await).into())
        })) else {
            panic!("the work asks nothing")
        };
        let [need]: [Need; 1] = needs.try_into().unwrap();
        let done = task::resume(task, vec![(need.id, Reply::Result(answer))]);
        let Step::Done(Answer::Result(JsValue::String(text))) = done else {
            panic!("the work does not end at the answer: {done:?}")
        };
        (need, text)
    }

    /// A need of port 7 as the host reads it.
    pub(crate) fn need(method: &str, args: Vec<JsValue>) -> Need {
        Need {
            id: 0,
            port: 7,
            method: method.to_owned(),
            args,
        }
    }

    /// A JSON text as a value.
    pub(crate) fn json(text: &str) -> JsValue {
        crate::js::json_parse::json_parse(text).unwrap()
    }
}
