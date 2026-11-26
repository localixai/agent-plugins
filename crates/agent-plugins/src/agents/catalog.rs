//! The subagents a session can run, whatever they came from (`src/agents/catalog/` in the npm
//! package).

pub mod builtin;
pub mod invoke;
mod mention_scan;
pub mod types;
