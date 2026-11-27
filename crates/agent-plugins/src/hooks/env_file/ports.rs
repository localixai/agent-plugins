//! Port of `src/hooks/env-file/ports.ts`, switched.
//!
//! The port of `CLAUDE_ENV_FILE` (hooks.md, "Persist environment variables"): the host owns the files,
//! the kit decides which one an event's hooks write and when one is cleared. Two files, because their
//! lifetimes differ: what `SessionStart` and `Setup` hooks write lives the session, and what
//! `CwdChanged` and `FileChanged` hooks write lives "until the next CwdChanged event, when Claude Code
//! clears them".
//!
//! One store serves every session of a host: each call names the session (`sessionId`, the session's
//! `CLAUDE_CODE_SESSION_ID`), because the file is the session's ("the shell commands Claude runs later
//! in the session"). Judgment (spec/judgments.md): the docs describe one session per process; a host
//! that runs many keys the files by the session id, so two sessions never share a file and a resumed
//! session (same id) finds its own again. The store itself (`EnvFileStore`) is the host's
//! interface: the core keeps the kinds, and the trait the host's object is read as.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// One of `ENV_FILE_KINDS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvFileKind {
    /// What lives the session.
    Session,
    /// What lives until the next `CwdChanged` event.
    Cwd,
}

/// The two files, in the package's order.
pub const ENV_FILE_KINDS: [EnvFileKind; 2] = [EnvFileKind::Session, EnvFileKind::Cwd];

/// The host's files of `CLAUDE_ENV_FILE`, one store for every session.
pub trait EnvFileStore: Send + Sync {
    /// The path of the session's file of one kind: a path a hook's `>> "$CLAUDE_ENV_FILE"` can append
    /// to, its folder made. The same path every call for one session and kind; never another session's.
    fn path<'a>(
        &'a self,
        session_id: &'a str,
        kind: EnvFileKind,
        signal: &'a Signal,
    ) -> BoxFuture<'a, String>;
    /// The text of a file (one of `path`'s, or the script `CLAUDE_ENV_FILE` named at launch); absent when there is none.
    fn read<'a>(&'a self, path: &'a str, signal: &'a Signal) -> BoxFuture<'a, Option<String>>;
    /// Empties the session's file of one kind, as a `CwdChanged` event does.
    fn clear<'a>(
        &'a self,
        session_id: &'a str,
        kind: EnvFileKind,
        signal: &'a Signal,
    ) -> BoxFuture<'a, ()>;
}

/// A value of the wire, `undefined` when it cannot be written.
fn wire(value: &impl Serialize) -> JsValue {
    to_js_value(value).unwrap_or_default()
}

/// The host's store, each method asked of it by its TypeScript name.
impl EnvFileStore for Port {
    fn path<'a>(
        &'a self,
        session_id: &'a str,
        kind: EnvFileKind,
        signal: &'a Signal,
    ) -> BoxFuture<'a, String> {
        let asked =
            self.method::<String>("path", vec![session_id.into(), wire(&kind), wire(signal)]);
        Box::pin(asked)
    }

    fn read<'a>(&'a self, path: &'a str, signal: &'a Signal) -> BoxFuture<'a, Option<String>> {
        let asked = self.method::<Option<String>>("read", vec![path.into(), wire(signal)]);
        Box::pin(asked)
    }

    fn clear<'a>(
        &'a self,
        session_id: &'a str,
        kind: EnvFileKind,
        signal: &'a Signal,
    ) -> BoxFuture<'a, ()> {
        let asked =
            self.method::<JsValue>("clear", vec![session_id.into(), wire(&kind), wire(signal)]);
        Box::pin(async move {
            asked.await;
        })
    }
}

crate::registry::exports! {
    "hooks/env-file/ports" {
        "ENV_FILE_KINDS" => ok || ENV_FILE_KINDS;
    }
}
