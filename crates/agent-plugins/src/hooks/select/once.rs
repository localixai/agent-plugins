//! Port of `src/hooks/select/once.ts`, switched.
//!
//! The `once` rule and the claims of a session's `once` handlers. The TypeScript told registrations
//! apart by identity (a `WeakSet`); here each is the serial it was registered under (the session's
//! registry, `host/session_core.rs`, or the engine's table of a host's registry, `hooks/engine.rs`),
//! which a copy keeps (docs/RUST.md, "The host session").

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

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

/// `isOnce` of a registered handler as it is held.
fn is_once_value(entry: &JsValue) -> bool {
    OnceEntry::deserialize(entry.clone()).is_ok_and(|entry| is_once(&entry))
}

/// Whether the session removes this handler after the run: `once` is set, the source honors it,
/// and the run succeeded. "A run that fails, blocks with exit code 2, or times out leaves the hook
/// in place, so it runs again on the next matching event."
#[must_use]
pub fn consumed_by_run(entry: &OnceEntry, succeeded: bool) -> bool {
    succeeded && is_once(entry)
}

/// The serials held by a running dispatch and spent by a successful run.
#[derive(Debug, Default)]
struct Holds {
    held: BTreeSet<u32>,
    spent: BTreeSet<u32>,
}

/// The `once` handlers of a session: held while a dispatch runs one, spent after its first
/// successful run (a background one when its process ends). `once` "removes the hook after its
/// first successful run", so a dispatch that starts while another runs the hook leaves it out, and
/// a run that fails, blocks or times out frees it again. Registrations are told apart by serial,
/// not by id: a hook registered again under the same id is a new one (judgment, spec/judgments.md).
#[derive(Debug, Default)]
pub struct OnceClaims {
    claims: Mutex<Holds>,
}

impl OnceClaims {
    fn with<R>(&self, body: impl FnOnce(&mut Holds) -> R) -> R {
        body(&mut self.claims.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// `selected` without the `once` handlers another run holds or a run spent; it holds the rest.
    pub fn take(&self, selected: Vec<(u32, JsValue)>) -> Vec<(u32, JsValue)> {
        self.with(|claims| {
            let free: Vec<(u32, JsValue)> = selected
                .into_iter()
                .filter(|(serial, entry)| {
                    !is_once_value(entry)
                        || !(claims.held.contains(serial) || claims.spent.contains(serial))
                })
                .collect();
            for (serial, entry) in &free {
                if is_once_value(entry) {
                    claims.held.insert(*serial);
                }
            }
            free
        })
    }

    /// Ends the hold of one run: spent after a success, free to run again otherwise. Whether it
    /// was spent.
    pub fn settle(&self, serial: u32, entry: &JsValue, succeeded: bool) -> bool {
        let spent = succeeded && is_once_value(entry);
        self.with(|claims| {
            claims.held.remove(&serial);
            if spent {
                claims.spent.insert(serial);
            }
        });
        spent
    }
}

/// The claims a dispatch reads: an engine's own (`OnceClaims`), or a session's, which also drops
/// a spent registration from its registry (`host/session_core.rs`).
pub trait Claims: Send + Sync {
    /// `OnceClaims.take`.
    fn take(&self, selected: Vec<(u32, JsValue)>) -> Vec<(u32, JsValue)>;
    /// `OnceClaims.settle`.
    fn settle(&self, serial: u32, entry: &JsValue, succeeded: bool);
}

impl Claims for OnceClaims {
    fn take(&self, selected: Vec<(u32, JsValue)>) -> Vec<(u32, JsValue)> {
        Self::take(self, selected)
    }

    fn settle(&self, serial: u32, entry: &JsValue, succeeded: bool) {
        Self::settle(self, serial, entry, succeeded);
    }
}

crate::registry::exports! {
    "hooks/select/once" {
        "consumedByRun" => ok |entry: OnceEntry, succeeded: bool| consumed_by_run(&entry, succeeded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    fn entry(once: bool) -> JsValue {
        json_parse(&format!(
            r#"{{"id":"s:1","handler":{{"type":"command","once":{once}}},"source":{{"kind":"skill"}}}}"#
        ))
        .unwrap_or_default()
    }

    #[test]
    fn a_held_once_handler_is_left_out_until_it_settles() {
        let claims = OnceClaims::default();
        let selected = vec![(1, entry(true)), (2, entry(false))];
        assert_eq!(claims.take(selected.clone()).len(), 2);
        assert_eq!(claims.take(selected.clone()).len(), 1);
        assert!(!claims.settle(1, &entry(true), false));
        assert_eq!(claims.take(selected.clone()).len(), 2);
        assert!(claims.settle(1, &entry(true), true));
        assert_eq!(claims.take(selected).len(), 1);
    }
}
