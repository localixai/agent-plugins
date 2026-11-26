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
//! session (same id) finds its own again. The store itself (`EnvFileStore`) is the host's interface
//! and stays TypeScript; the core keeps the kinds.

use serde::{Deserialize, Serialize};

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

crate::registry::exports! {
    "hooks/env-file/ports" {
        "ENV_FILE_KINDS" => ok || ENV_FILE_KINDS;
    }
}
