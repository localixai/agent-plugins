//! The core's counterpart of `src/ports/failure.ts` at sha256:7c8920dcd5e9, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! How a port reports an effect that did not happen (a connection refused, a timeout, a model
//! that failed): a value in the result, never a panic.

use serde::{Deserialize, Serialize};

/// An effect that did not happen: `failure` is its kind (`connection`, `timeout`, `aborted`…,
/// each port names its own), `message` what the host saw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortFailure {
    /// The kind of failure, from the set the port names.
    pub failure: String,
    /// What the host saw, for the person.
    pub message: String,
}
