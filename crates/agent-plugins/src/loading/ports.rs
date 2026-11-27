//! The core's counterpart of `src/loading/ports.ts` at sha256:5c7553da3d01, which stays TypeScript: the host's interface (docs/RUST.md, "What stays TypeScript").
//!
//! The loading stream's own port (docs/ARCHITECTURE.md, "Ports"): it lives here, not in `ports/`,
//! because the stream owns it.

use crate::js::{JsValue, is_truthy};
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// Whether the user accepted the workspace trust dialog for a folder (permissions.md, "Project allow
/// rules and workspace trust"). The host keeps the answer (`projects["<path>"].hasTrustDialogAccepted`
/// in `~/.claude.json`, or the session's memory for the home directory); `path` is the key
/// `workspaceTrustKey` names, so the host never decides what a key is. Reading it is an effect: the
/// answer is a promise, and a host that cannot read it answers `false`.
pub trait WorkspaceTrust: Send + Sync {
    /// Whether the dialog was accepted for `path`.
    fn is_folder_trusted<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, bool>;
}

impl WorkspaceTrust for Port {
    fn is_folder_trusted<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, bool> {
        let signal = signal
            .and_then(|signal| crate::js::value::to_js_value(signal).ok())
            .unwrap_or(JsValue::Undefined);
        let asked = self.method::<JsValue>("isFolderTrusted", vec![path.into(), signal]);
        Box::pin(async move { is_truthy(&asked.await) })
    }
}
