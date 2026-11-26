//! agent-plugins compiled to WebAssembly: [`agent_plugins::registry::call`] behind a small ABI
//! ([`abi`]), so a host in another language calls every export by name, JSON in and out. The npm
//! package's wrapper (`src/wasm/`) is one such host. Nothing is kept between calls.

pub mod abi;
