//! The core's counterpart of `src/ports/environment.ts` at sha256:05cd4f14ca6c, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! The process environment as the host gives it: the crate reads no variable itself.

use indexmap::IndexMap;

/// Environment variables, read through the host.
pub trait Environment: Send + Sync {
    /// The variable's value, or `None` when it is not set.
    fn get(&self, name: &str) -> Option<String>;
    /// Every variable, in the host's order.
    fn snapshot(&self) -> IndexMap<String, String>;
}
