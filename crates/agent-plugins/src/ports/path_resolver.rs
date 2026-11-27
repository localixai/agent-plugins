//! The core's counterpart of `src/ports/path-resolver.ts` at sha256:be7c8347d0a5, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! A host path with its links resolved.

use super::{BoxFuture, PortFailure, Signal};

/// The host's `realpath`.
pub trait PathResolver: Send + Sync {
    /// The path with every link resolved; `None` when it does not exist.
    fn realpath<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Result<String, PortFailure>>>;
}
