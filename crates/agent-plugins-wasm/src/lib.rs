//! agent-plugins compiled to WebAssembly: [`agent_plugins::registry::call`] behind a small ABI
//! ([`abi`]), so a host in another language calls every export by name, JSON in and out, and
//! drives an export that waits on it with `start` and `resume`. The npm package's wrapper
//! (`src/wasm/`) is one such host. Between calls the module keeps only the tasks that wait on the
//! host and the handles it holds.

pub mod abi;
