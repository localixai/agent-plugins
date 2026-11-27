//! The core's counterpart of `src/hooks/select/once.ts` at sha256:34446d70f0fc, which stays TypeScript:
//! its claims hold the host's registrations by identity (`WeakSet`), and a copy is not that object
//! (docs/RUST.md, "What stays TypeScript"). Only `consumedByRun`, a rule over data, is here, for the
//! core's assembly of an outcome.

use serde::Deserialize;

use crate::hooks::contracts::HookSource;
use crate::hooks::source_kind::policy_of;
use crate::js::JsValue;

/// The part of a registered handler `once` reads.
#[derive(Clone, Debug, Deserialize)]
pub struct OnceEntry {
    /// The handler's configuration.
    #[serde(default)]
    pub handler: JsValue,
    /// Where it was declared.
    pub source: HookSource,
}

/// `once` is set and the source honors it (`SourcePolicy.honorsOnce`).
fn is_once(entry: &OnceEntry) -> bool {
    entry.handler.get("once").and_then(JsValue::as_bool) == Some(true)
        && policy_of(entry.source.kind).honors_once
}

/// Whether the session removes this handler after the run: `once` is set, the source honors it,
/// and the run succeeded. "A run that fails, blocks with exit code 2, or times out leaves the hook
/// in place, so it runs again on the next matching event."
#[must_use]
pub fn consumed_by_run(entry: &OnceEntry, succeeded: bool) -> bool {
    succeeded && is_once(entry)
}
