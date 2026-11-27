//! Claude Code plugin support for any agent written in Rust: the Rust implementation of
//! [Agent Plugins Extended](https://github.com/localixai/agent-plugins-extended), the same
//! specification the npm package `@localix-ai/agent-plugins` beside it implements, at the same
//! pinned revision (`spec/SOURCE.json`).
//!
//! The crate reads, validates and decides; it does no I/O. Every effect (a file read, a command,
//! a request, the clock) is a port, a trait the host implements. Its modules are the npm
//! package's, layer for layer (docs/ARCHITECTURE.md), so one behaviour has one home in both: a
//! function here answers what its namesake there answers, which the differential tests hold it to
//! (docs/RUST.md).

pub mod agents;
pub mod cli;
pub mod convert;
pub mod format;
pub mod foundation;
pub mod hooks;
pub mod host;
pub mod js;
pub mod loading;
pub mod marketplace;
pub mod ports;
pub mod protocol;
pub mod registry;
pub mod rules;
pub mod semver;
pub mod skills;
pub mod updates;
mod yaml;
pub mod zod;
