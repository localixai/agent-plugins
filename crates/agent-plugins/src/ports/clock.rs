//! The core's counterpart of `src/ports/clock.ts` at sha256:f985dde4b4a4, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! Time, the only way the crate sees it: no `SystemTime`, no sleeping in the crate. A host passes
//! the real clock; a test passes a fixed one and moves time by hand.

use super::{BoxFuture, Signal};

/// What cancels a timer set with [`Clock::set_timer`].
pub type CancelTimer = Box<dyn FnOnce() + Send>;

/// The host's clock.
pub trait Clock: Send + Sync {
    /// Milliseconds since the Unix epoch.
    fn now(&self) -> f64;
    /// Runs `fire` once after `ms`; the returned function cancels it.
    fn set_timer(&self, fire: Box<dyn FnOnce() + Send>, ms: f64) -> CancelTimer;
    /// Resolves after `ms`, or early with `Err` carrying the abort when `signal` aborts first.
    fn sleep<'a>(
        &'a self,
        ms: f64,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Result<(), String>>;
}
