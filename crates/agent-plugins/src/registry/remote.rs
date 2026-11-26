//! The host's values inside the core, for an export a host drives (`registry::start`): what
//! cannot be copied as data crosses as a number the host keeps it under, and each use of it is a
//! need (`registry::task`).
//!
//! - A port, `{"$port": n}`, is a [`Port`]: it implements the port traits, each method a need
//!   named as the TypeScript port's method, with the same arguments (`files.readText(path,
//!   signal)` is the need `readText` with `[path, signal]`).
//! - A callback, `{"$fn": n}`, is a [`Callback`]: calling it is the need `call`.
//! - An `AbortSignal`, `{"$signal": n, "aborted": bool}`, is a [`Signal`] whose flag the host
//!   sets when it resumes a task (`"aborted": [n]`); passed to a port, it crosses back as
//!   `{"$signal": n, "aborted": bool}` and is the host's own signal again. A signal the core made
//!   itself crosses as `{"$signal": null, "aborted": bool}`, a new one on the host's side. The
//!   core never waits on an abort: whatever waits is the host's, and the host's port watches its
//!   own signal.

use std::collections::BTreeMap;
use std::future::{Future, pending};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use serde::de::{DeserializeOwned, Deserializer};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use super::task;
use super::wire::Bytes;
use crate::js::JsValue;
use crate::ports::files::{DirectoryEntry, FileStat, PluginFiles, TextFileReader};
use crate::ports::{BoxFuture, Signal};

/// The host's answer to `method` of value `id`, read as `R`. An answer `R` cannot hold ends the
/// task with a fault: the host broke the port's contract.
fn answered<R: DeserializeOwned + Send + 'static>(
    id: u32,
    method: &'static str,
    args: Vec<JsValue>,
) -> impl Future<Output = R> + Send + 'static {
    let asked = task::ask(id, method, args);
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

/// An argument of a need: a path, a signal or `undefined`.
fn arg<T: Serialize>(value: Option<&T>) -> JsValue {
    value
        .and_then(|value| crate::js::value::to_js_value(value).ok())
        .unwrap_or_default()
}

impl Port {
    /// `method(path, signal)` of the host's port, read as `R`.
    fn at<R: DeserializeOwned + Send + 'static>(
        self,
        method: &'static str,
        path: &str,
        signal: Option<&Signal>,
    ) -> BoxFuture<'static, R> {
        Box::pin(answered(
            self.id,
            method,
            vec![JsValue::from(path), arg(signal)],
        ))
    }
}

impl TextFileReader for Port {
    fn read_text<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<String>> {
        self.at("readText", path, signal)
    }
}

impl PluginFiles for Port {
    fn read_bytes<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<u8>>> {
        let bytes = self.at::<Option<Bytes>>("readBytes", path, signal);
        Box::pin(async move { bytes.await.map(|Bytes(bytes)| bytes) })
    }

    fn list<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<DirectoryEntry>>> {
        self.at("list", path, signal)
    }

    fn stat<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<FileStat>> {
        self.at("stat", path, signal)
    }

    fn exists<'a>(&'a self, path: &'a str, signal: Option<&'a Signal>) -> BoxFuture<'a, bool> {
        self.at("exists", path, signal)
    }
}
