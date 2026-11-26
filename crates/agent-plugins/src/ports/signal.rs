//! A request's cancellation, the npm package's `AbortSignal`: the host aborts it, a port that
//! does I/O watches it and answers its `aborted` failure.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A cancellation flag shared by the host and the ports it hands it to. A clone watches the same
/// flag.
#[derive(Clone, Debug, Default)]
pub struct Signal {
    aborted: Arc<AtomicBool>,
}

impl Signal {
    /// A signal that has not been aborted.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Aborts every clone of this signal.
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::SeqCst);
    }

    /// Whether the signal was aborted.
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        self.aborted.load(Ordering::SeqCst)
    }
}
