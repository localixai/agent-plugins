//! The hook engine (`src/hooks/` in the npm package): what a hook's JSON output and exit
//! code mean, and what its input carries. Only the folders already in the core are declared.

pub mod contracts;
pub mod core;
pub mod env_file;
pub mod handlers;
pub mod input;
pub mod output;
pub mod register;
pub mod select;
pub mod source_kind;
pub mod verdict;
