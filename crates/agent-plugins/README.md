# agent-plugins (Rust)

Claude Code plugin support for any agent: the implementation of
[Agent Plugins Extended](https://github.com/localixai/agent-plugins-extended). A Rust host takes
the crate; the npm package `@localix-ai/agent-plugins` in the same repository is a thin wrapper
over the same crate compiled to WebAssembly (`crates/agent-plugins-wasm`), which any language
with a WebAssembly runtime can load.

Work in progress: the package moves into the crate module by module, and each moved module runs
the package's whole test suite on the Rust; [COVERAGE.md](COVERAGE.md) shows how far it has come
(docs/RUST.md in the repository).

The crate reads, validates and decides; it does no I/O. Files, commands, requests, the
environment and the clock are ports: traits the host implements, whose methods return futures
that name no runtime.

```toml
[dependencies]
agent-plugins = { git = "https://github.com/localixai/agent-plugins", tag = "rust-v0.1.0" }
```

No release is tagged yet.
