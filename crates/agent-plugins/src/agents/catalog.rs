//! The subagents a session can run, whatever they came from (`src/agents/catalog/` in the npm
//! package).

pub mod builtin;
pub mod invoke;
mod mention_scan;
pub mod resolve;
pub mod scan;
pub mod scopes;
pub mod spawn;
pub mod types;
