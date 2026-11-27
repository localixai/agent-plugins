//! Port of `src/marketplace/copy-sweep.ts`, switched.
//!
//! The dated copies beside `installed_plugins.json` age out (plugins/loading.md, "Find plugins on
//! disk"; plugins/troubleshooting.md: "the copies age out on the `cleanupPeriodDays` schedule").
//! The setting: "Claude Code deletes the files in the paths below once they're older than
//! `cleanupPeriodDays` ... The default is 30 days and the minimum is 1; setting `0` fails with a
//! validation error" (claude-directory.md, "Cleaned up automatically").
//!
//! Judgments (spec/judgments.md): a copy is removed once it is strictly older than the period, by
//! the time it was last written; a period that is not a whole number of at least 1 is the
//! default, since the setting would not have validated; only the two kinds of dated copy are ever
//! removed.

use serde::{Deserialize, Serialize};

use super::command_source::WithSignal;
use super::store_layout::{StoreEntry, classify_store_entry};
use crate::js::{is_integer, promise_all};
use crate::ports::clock::Clock;
use crate::ports::failure::PortFailure;
use crate::ports::marketplace::PluginStateStore;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// The period when the setting gives none that validates.
pub const DEFAULT_CLEANUP_PERIOD_DAYS: f64 = 30.0;

const DAY_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

fn period_ms(days: Option<f64>) -> f64 {
    days.filter(|days| is_integer(*days) && *days >= 1.0)
        .unwrap_or(DEFAULT_CLEANUP_PERIOD_DAYS)
        * DAY_MS
}

fn is_dated_copy(name: &str) -> bool {
    let kind = classify_store_entry(name);
    kind == StoreEntry::SetAsideCopy || kind == StoreEntry::UnreadableCopy
}

/// A name directly under the plugins root and when it was last written, in Unix milliseconds.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgedName {
    /// The name.
    pub name: String,
    /// When it was last written.
    pub modified_at_ms: f64,
}

/// What `agedCopies` reads.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgedCopiesInput {
    /// The names and their ages.
    pub entries: Vec<AgedName>,
    /// Now, in Unix milliseconds.
    pub now_ms: f64,
    /// The setting.
    #[serde(default)]
    pub cleanup_period_days: Option<f64>,
}

/// The dated copies among `entries` that are older than `cleanupPeriodDays` at `nowMs`.
#[must_use]
pub fn aged_copies(input: &AgedCopiesInput) -> Vec<String> {
    let limit = period_ms(input.cleanup_period_days);
    input
        .entries
        .iter()
        .filter(|one| is_dated_copy(&one.name) && input.now_ms - one.modified_at_ms > limit)
        .map(|one| one.name.clone())
        .collect()
}

/// What a sweep did.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CopySweep {
    /// The copies removed.
    pub removed: Vec<String>,
    /// The removals that failed; the copies stay and are tried again on the next sweep.
    pub failures: Vec<PortFailure>,
}

/// The ports a sweep uses.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SweepPorts {
    /// The install records.
    pub store: Port,
    /// The clock.
    pub clock: Port,
}

/// The sweep's options.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SweepOptions {
    /// The setting.
    #[serde(default)]
    pub cleanup_period_days: Option<f64>,
}

/// Lists the root, looks up the age of each dated copy, and removes those that aged out.
pub async fn sweep_aged_copies(
    store: &dyn PluginStateStore,
    clock: &dyn Clock,
    options: &SweepOptions,
    signal: &Signal,
) -> CopySweep {
    let names = store
        .names(signal)
        .await
        .into_iter()
        .filter(|name| is_dated_copy(name));
    let lookups: Vec<BoxFuture<'_, (String, Option<f64>)>> = names
        .map(|name| -> BoxFuture<'_, (String, Option<f64>)> {
            Box::pin(async move {
                let modified_at_ms = store.modified_at(&name, signal).await;
                (name, modified_at_ms)
            })
        })
        .collect();
    let found = promise_all(lookups).await;
    let entries = found
        .into_iter()
        .filter_map(|(name, modified_at_ms)| {
            modified_at_ms.map(|modified_at_ms| AgedName {
                name,
                modified_at_ms,
            })
        })
        .collect();
    let aged = aged_copies(&AgedCopiesInput {
        entries,
        now_ms: clock.now().await,
        cleanup_period_days: options.cleanup_period_days,
    });
    let removals: Vec<BoxFuture<'_, (String, Result<(), PortFailure>)>> = aged
        .into_iter()
        .map(|name| -> BoxFuture<'_, (String, Result<(), PortFailure>)> {
            Box::pin(async move {
                let result = store.remove(&name, signal).await;
                (name, result)
            })
        })
        .collect();
    let outcomes = promise_all(removals).await;
    let mut sweep = CopySweep {
        removed: Vec::new(),
        failures: Vec::new(),
    };
    for (name, result) in outcomes {
        match result {
            Ok(()) => sweep.removed.push(name),
            Err(failure) => sweep.failures.push(failure),
        }
    }
    sweep
}

crate::registry::exports! {
    "marketplace/copy-sweep" {
        "DEFAULT_CLEANUP_PERIOD_DAYS" => ok || DEFAULT_CLEANUP_PERIOD_DAYS;
        "agedCopies" => ok |input: AgedCopiesInput| aged_copies(&input);
        "sweepAgedCopies" => async ok |ports: SweepPorts, options: SweepOptions, with: WithSignal|
            sweep_aged_copies(&ports.store, &ports.clock, &options, &with.signal).await;
    }
}
