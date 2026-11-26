//! Layer 1: the ports, the traits a host implements for every effect the crate needs
//! (`src/ports/` in the npm package). One file per port.
//!
//! The npm package's ports return promises; here a port method returns a [`BoxFuture`], a boxed
//! future that names no runtime. A host on an async runtime runs the crate's futures on it, and
//! hooks that the docs run in parallel run in parallel; a host with none drives them with any
//! `block_on`, and its ports may simply block. The crate itself never spawns, sleeps or blocks.
//! An `AbortSignal` of the npm package is a [`Signal`].

pub mod clock;
pub mod command_runner;
pub mod environment;
pub mod failure;
pub mod files;
pub mod folder_scan;
pub mod http_client;
pub mod locations;
pub mod marketplace;
pub mod mcp_caller;
pub mod model_runner;
pub mod path_resolver;
pub mod scratch_files;
pub mod signal;

use std::future::Future;
use std::pin::Pin;

pub use failure::PortFailure;
pub use signal::Signal;

/// The future a port method returns: boxed so the traits stay object safe, `Send` so a host on a
/// multi-threaded runtime can run it anywhere.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
