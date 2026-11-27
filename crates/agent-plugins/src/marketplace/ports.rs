//! The core's counterpart of `src/marketplace/ports.ts` at sha256:7c8fdb6a4f31, which stays TypeScript: the host's interfaces (docs/RUST.md, "What stays TypeScript").
//!
//! The ports of the install flow, the ones the stream owns (docs/ARCHITECTURE.md, "Ports"): the
//! directory a host staged a fetched plugin in. Only what the core uses is here.

use serde::Serialize;

use super::install_failures::CommitFailure;
use crate::js::JsValue;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// One install of a staged directory into the cache (`StagedCommit`): the members the host was
/// given, as the TypeScript passes them.
#[derive(Clone, Debug, Serialize)]
pub struct StagedCommit<'a> {
    /// Where the host staged the plugin.
    #[serde(rename = "stagedPath")]
    pub staged_path: &'a str,
    /// A version folder of the cache.
    pub destination: &'a str,
    /// What the copy does with each symbolic link (`planTreeCopy`); absent when the staged tree
    /// holds no link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<&'a JsValue>,
}

/// A directory a host staged a fetched plugin in, so it installs by moving that directory rather
/// than copying files the kit read.
pub trait StagedDirectory: Send + Sync {
    /// Installs the directory at `destination` so a reader sees the old copy or the new, never a
    /// mix; `None` on success.
    fn commit<'a>(
        &'a self,
        request: &StagedCommit<'_>,
        signal: &'a Signal,
    ) -> BoxFuture<'a, Option<CommitFailure>>;
}

impl StagedDirectory for Port {
    fn commit<'a>(
        &'a self,
        request: &StagedCommit<'_>,
        signal: &'a Signal,
    ) -> BoxFuture<'a, Option<CommitFailure>> {
        let request = crate::js::value::to_js_value(request).unwrap_or(JsValue::Undefined);
        let signal = crate::js::value::to_js_value(signal).unwrap_or(JsValue::Undefined);
        Box::pin(self.method::<Option<CommitFailure>>("commit", vec![request, signal]))
    }
}

/// Opens a directory a source named (`PluginDirectories`): read-only plugin files rooted there,
/// so the kit can judge what is inside without reading outside it. `None` when the path is not an
/// existing directory.
pub trait PluginDirectories: Send + Sync {
    /// The files under `path`.
    fn open<'a>(&'a self, path: &'a str, signal: &'a Signal) -> BoxFuture<'a, Option<Port>>;
}

impl PluginDirectories for Port {
    fn open<'a>(&'a self, path: &'a str, signal: &'a Signal) -> BoxFuture<'a, Option<Port>> {
        let signal = crate::js::value::to_js_value(signal).unwrap_or(JsValue::Undefined);
        Box::pin(self.method::<Option<Port>>("open", vec![JsValue::from(path), signal]))
    }
}
