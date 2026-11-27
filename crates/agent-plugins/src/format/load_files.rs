//! Ported from `src/format/load-files.ts`; only the core uses it now, so it has no wrapper.
//!
//! The plugin folder as one load reads it, and [`Files`], the folder as the core's loading holds
//! it: the host's port, or a load's view of it, so one load's readers share one view.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::js::SharedPromise;
use crate::ports::files::{DirectoryEntry, FileStat, PluginFiles, TextFileReader};
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// A plugin folder the core reads: the host's port as it crossed (`{"$port": n}`), or files the
/// core holds itself (a load's view, a Rust host's own folder).
#[derive(Clone)]
pub enum Files {
    /// The host's `PluginFiles`.
    Host(Port),
    /// Files the core holds.
    Core(Arc<dyn PluginFiles>),
}

impl fmt::Debug for Files {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(port) => port.fmt(formatter),
            Self::Core(_) => formatter.write_str("Files::Core"),
        }
    }
}

impl From<Port> for Files {
    fn from(port: Port) -> Self {
        Self::Host(port)
    }
}

impl<'de> Deserialize<'de> for Files {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Port::deserialize(deserializer).map(Self::Host)
    }
}

impl Serialize for Files {
    /// The host's port as it crossed; files the core holds never cross.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Host(port) => port.serialize(serializer),
            Self::Core(_) => Err(serde::ser::Error::custom(
                "the core's own files do not cross",
            )),
        }
    }
}

impl Files {
    fn inner(&self) -> &dyn PluginFiles {
        match self {
            Self::Host(port) => port,
            Self::Core(files) => files.as_ref(),
        }
    }
}

impl TextFileReader for Files {
    fn read_text<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<String>> {
        self.inner().read_text(path, signal)
    }
}

impl PluginFiles for Files {
    fn read_bytes<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<u8>>> {
        self.inner().read_bytes(path, signal)
    }

    fn list<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<DirectoryEntry>>> {
        self.inner().list(path, signal)
    }

    fn stat<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<FileStat>> {
        self.inner().stat(path, signal)
    }

    fn exists<'a>(&'a self, path: &'a str, signal: Option<&'a Signal>) -> BoxFuture<'a, bool> {
        self.inner().exists(path, signal)
    }
}

/// What one load asked of `stat` and `exists`, by path: the host's answer, shared by every caller.
type Known<T> = Mutex<HashMap<String, SharedPromise<T>>>;

/// The port as one load reads it (`loadFiles`' object).
struct LoadFiles {
    files: Files,
    signal: Option<Signal>,
    stats: Known<Option<FileStat>>,
    exists: Known<bool>,
}

impl LoadFiles {
    /// `signal?.throwIfAborted()` before each call reaches the host.
    async fn checked(&self) {
        if let Some(signal) = &self.signal {
            signal.throw_if_aborted().await;
        }
    }

    /// The host's answer for `path`, asked once per load (`once`).
    fn once<T: Clone + Send + 'static>(
        &self,
        known: &Known<T>,
        path: &str,
        ask: impl FnOnce() -> BoxFuture<'static, T>,
    ) -> SharedPromise<T> {
        let mut known = known.lock().unwrap_or_else(PoisonError::into_inner);
        known
            .entry(path.to_owned())
            .or_insert_with(|| SharedPromise::new(ask()))
            .clone()
    }
}

impl TextFileReader for LoadFiles {
    fn read_text<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            self.checked().await;
            self.files.read_text(path, signal).await
        })
    }
}

impl PluginFiles for LoadFiles {
    fn read_bytes<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<u8>>> {
        Box::pin(async move {
            self.checked().await;
            self.files.read_bytes(path, signal).await
        })
    }

    fn list<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<DirectoryEntry>>> {
        Box::pin(async move {
            self.checked().await;
            self.files.list(path, signal).await
        })
    }

    fn stat<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<FileStat>> {
        Box::pin(async move {
            self.checked().await;
            let (files, owned, inner) = (self.files.clone(), path.to_owned(), signal.cloned());
            let ask = move || -> BoxFuture<'static, Option<FileStat>> {
                Box::pin(async move { files.stat(&owned, inner.as_ref()).await })
            };
            self.once(&self.stats, path, ask).wait().await
        })
    }

    fn exists<'a>(&'a self, path: &'a str, signal: Option<&'a Signal>) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            self.checked().await;
            let (files, owned, inner) = (self.files.clone(), path.to_owned(), signal.cloned());
            let ask = move || -> BoxFuture<'static, bool> {
                Box::pin(async move { files.exists(&owned, inner.as_ref()).await })
            };
            self.once(&self.exists, path, ask).wait().await
        })
    }
}

/// The port as one load reads it (judgment):
/// - once `signal` aborts, every next call rejects with its reason before reaching the host, so the
///   parallel branches of a load stop at their next read instead of reading the rest of the plugin;
/// - `stat` and `exists` of a path are asked of the host once per load: discovery, containment and
///   link checks ask about the same folders, and a remote port pays a round trip for each. Contents
///   (`readText`, `readBytes`, `list`) are always read from the host.
#[must_use]
pub fn load_files(files: Files, signal: Option<Signal>) -> Files {
    Files::Core(Arc::new(LoadFiles {
        files,
        signal,
        stats: Mutex::default(),
        exists: Mutex::default(),
    }))
}
