//! The core's counterpart of `src/ports/environment.ts` at sha256:05cd4f14ca6c, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! The process environment as the host gives it: the crate reads no variable itself.

use indexmap::IndexMap;

use super::BoxFuture;

/// Environment variables, read through the host. Its methods answer at once in the TypeScript;
/// here they return futures, so a host across the boundary answers each as a need
/// (`crate::registry`), and a Rust host returns ready ones.
pub trait Environment: Send + Sync {
    /// The variable's value, or `None` when it is not set.
    fn get<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>>;
    /// Every variable, in the host's order.
    fn snapshot(&self) -> BoxFuture<'_, IndexMap<String, String>>;
}
