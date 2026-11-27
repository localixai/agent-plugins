//! Port of `src/host/background-queue.ts`, switched.
//!
//! What the background hooks of a session delivered since the host last asked (docs/API.md, "Host
//! and sessions", "Background hooks"): `settle` keeps one delivery as its promise settles and says
//! what to announce, `take` hands over what settled in the order it settled and forgets it. The
//! host's promises and listeners stay the edge's (`src/host/background-queue.ts`): it awaits each
//! promise and calls its listeners with what `settle` answers.
//!
//! Judgments (spec/judgments.md): the queue holds every delivery until the host takes it, so a host
//! that never takes them keeps them all; it is the host's to drain. The repeat of an async
//! `SessionStart` hook reaches it emptied (`session-start-repeats.ts`); `HookOutcome.background`
//! read directly hands over every delivery.

use std::sync::{Mutex, PoisonError};

use crate::hooks::output::background::{SettledDeliveries, group_deliveries};
use crate::hooks::verdict::types::BackgroundDelivery;
use crate::registry::Handle;

/// The deliveries that settled and were not taken yet.
#[derive(Debug, Default)]
pub struct BackgroundQueue {
    settled: Mutex<Vec<BackgroundDelivery>>,
}

fn carries_something(delivered: &SettledDeliveries) -> bool {
    delivered.additional_context.len() + delivered.system_messages.len() > 0
        || delivered.wake.is_some()
}

impl BackgroundQueue {
    /// Keeps a delivery that settled; what the listeners hear of it, `None` when it carries nothing.
    pub fn settle(&self, delivery: BackgroundDelivery) -> Option<SettledDeliveries> {
        let delivered = group_deliveries(std::slice::from_ref(&delivery));
        self.settled
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(delivery);
        carries_something(&delivered).then_some(delivered)
    }

    /// What settled since the last call, grouped, and forgotten.
    pub fn take(&self) -> SettledDeliveries {
        let settled =
            std::mem::take(&mut *self.settled.lock().unwrap_or_else(PoisonError::into_inner));
        group_deliveries(&settled)
    }
}

crate::registry::methods! {
    BackgroundQueue {
        "settle" => nullable |this, delivery: BackgroundDelivery| this.settle(delivery);
        "take" => ok |this| this.take();
    }
}

/// An empty queue.
#[must_use]
pub fn create_background_queue() -> BackgroundQueue {
    BackgroundQueue::default()
}

crate::registry::exports! {
    "host/background-queue" {
        "createBackgroundQueue" => ok || Handle::new(create_background_queue());
    }
}
