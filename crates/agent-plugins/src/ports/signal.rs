//! A request's cancellation, the npm package's `AbortSignal`: the host aborts it, a port that
//! does I/O watches it and answers its `aborted` failure.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A cancellation flag shared by the host and the ports it hands it to. A clone watches the same
/// flag.
#[derive(Clone, Debug, Default)]
pub struct Signal {
    aborted: Arc<AtomicBool>,
    /// The host's number for it when it came across the boundary (`{"$signal": n}`,
    /// `crate::registry`), so a port the host implements is handed the host's own signal back.
    host: Option<u32>,
}

impl Signal {
    /// A signal that has not been aborted.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The signal a host handed across the boundary as number `host`, aborted or not; its flag
    /// is the one `crate::registry` sets when the host says it aborted.
    pub(crate) fn from_host(host: u32, aborted: Arc<AtomicBool>) -> Self {
        Self {
            aborted,
            host: Some(host),
        }
    }

    /// The host's number for it, when it came across the boundary.
    pub(crate) const fn host(&self) -> Option<u32> {
        self.host
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
