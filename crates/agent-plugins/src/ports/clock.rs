//! The core's counterpart of `src/ports/clock.ts` at sha256:f985dde4b4a4, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! Time, the only way the crate sees it: no `SystemTime`, no sleeping in the crate. A host passes
//! the real clock; a test passes a fixed one and moves time by hand.

use super::{BoxFuture, Signal};

/// What cancels a timer set with [`Clock::set_timer`]: its future is done once the host has
/// cancelled it.
pub type CancelTimer = Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>;

/// The host's clock. A method the TypeScript answers at once returns a future too, so a host
/// across the boundary answers it as a need (`crate::registry`); a Rust host returns a ready one.
pub trait Clock: Send + Sync {
    /// Milliseconds since the Unix epoch.
    fn now(&self) -> BoxFuture<'_, f64>;
    /// Runs `fire` once after `ms`; the answer cancels it.
    fn set_timer(&self, fire: Box<dyn FnOnce() + Send>, ms: f64) -> BoxFuture<'_, CancelTimer>;
    /// Resolves after `ms`, or early with `Err` carrying the abort when `signal` aborts first.
    fn sleep<'a>(
        &'a self,
        ms: f64,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Result<(), String>>;
}
