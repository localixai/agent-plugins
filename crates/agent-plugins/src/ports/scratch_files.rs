//! The core's counterpart of `src/ports/scratch-files.ts` at sha256:f89696b1d8b7, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! Files the crate hands to the model instead of a text too long to show, such as a hook output
//! over its cap.

use super::BoxFuture;

/// Where the host keeps such files.
pub trait ScratchFiles: Send + Sync {
    /// Writes `content` under a name built from `name` and answers its host path.
    fn write<'a>(&'a self, name: &'a str, content: &'a str) -> BoxFuture<'a, String>;
}
