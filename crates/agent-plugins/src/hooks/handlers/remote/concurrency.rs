//! Port of `src/hooks/handlers/remote/concurrency.ts`, switched.
//!
//! hooks.md, "Hook handler fields" (the last paragraph under "Matcher patterns"): "When an event
//! fires, its matching hooks run in parallel. If more than 12 of them are prompt or agent hooks, 12
//! of those run at once and the rest start as running ones finish. Each waiting hook's `timeout`
//! starts counting when that hook starts."
//!
//! The timeout of a waiting hook starts when its run starts because the kit computes the limit
//! when it hands the request to the port, which enforces it from then on; a hook that waits for a
//! slot has no request yet.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

use crate::js::{JsValue, promise_all};
use crate::ports::BoxFuture;
use crate::registry::Callback;

/// At most this many prompt or agent hooks run at once.
pub const MAX_MODEL_HOOKS_AT_ONCE: f64 = 12.0;

/// The types the limit counts: "prompt or agent hooks".
const LIMITED: [(&str, bool); 5] = [
    ("command", false),
    ("http", false),
    ("mcp_tool", false),
    ("prompt", true),
    ("agent", true),
];

fn limited(entry: &JsValue) -> bool {
    let kind = entry
        .get("handler")
        .and_then(|handler| handler.get("type"))
        .and_then(JsValue::as_str);
    LIMITED
        .iter()
        .any(|(name, limited)| Some(*name) == kind && *limited)
}

fn locked<T, R>(cell: &Mutex<T>, body: impl FnOnce(&mut T) -> R) -> R {
    body(&mut cell.lock().unwrap_or_else(PoisonError::into_inner))
}

/// Runs `run` for every handler at once, except that at most `limit` prompt or agent handlers are
/// running at any moment; the rest start, in order, as running ones finish. Results come back in
/// the order of `entries`, so what merges them is the same from run to run.
pub async fn run_limited<'a, T, F>(entries: &'a [JsValue], run: F, limit: usize) -> Vec<T>
where
    T: Send + 'a,
    F: Fn(usize, &'a JsValue) -> BoxFuture<'a, T> + Sync + 'a,
{
    let results: Mutex<Vec<Option<T>>> = Mutex::new(entries.iter().map(|_| None).collect());
    let waiting: Mutex<VecDeque<usize>> = Mutex::new(VecDeque::new());
    let (results, waiting, run) = (&results, &waiting, &run);
    let settle = move |index: usize, entry: &'a JsValue| -> BoxFuture<'_, ()> {
        Box::pin(async move {
            let result = run(index, entry).await;
            locked(results, |results| {
                if let Some(slot) = results.get_mut(index) {
                    *slot = Some(result);
                }
            });
        })
    };
    let mut running: Vec<BoxFuture<'_, ()>> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if limited(entry) {
            locked(waiting, |waiting| waiting.push_back(index));
        } else {
            running.push(settle(index, entry));
        }
    }
    let slots = limit.min(locked(waiting, |waiting| waiting.len()));
    for _ in 0..slots {
        running.push(Box::pin(async move {
            while let Some(index) = locked(waiting, VecDeque::pop_front) {
                if let Some(entry) = entries.get(index) {
                    settle(index, entry).await;
                }
            }
        }));
    }
    promise_all(running).await;
    locked(results, std::mem::take)
        .into_iter()
        .flatten()
        .collect()
}

/// A limit as the host gave it: a count, or the default.
pub fn limit_of(limit: Option<f64>) -> usize {
    let limit = limit.unwrap_or(MAX_MODEL_HOOKS_AT_ONCE);
    if limit.is_finite() && limit >= 1.0 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a finite count of at least one, as `Math.min(limit, waiting.length)` reads it"
        )]
        let count = limit as usize;
        count
    } else {
        0
    }
}

crate::registry::exports! {
    "hooks/handlers/remote/concurrency" {
        "MAX_MODEL_HOOKS_AT_ONCE" => ok || MAX_MODEL_HOOKS_AT_ONCE;
        "runLimited" => async ok |entries: Vec<JsValue>, run: Callback, limit: Option<f64>|
            run_limited(&entries, |_, entry| Box::pin(run.call::<JsValue>(vec![entry.clone()])),
                limit_of(limit)).await;
    }
}
