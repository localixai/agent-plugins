import { coreModule } from '../wasm/bridge.ts';

/**
 * The LSP server configuration as Claude Code's schema check reads it, one rule set for the loader
 * and the validator (plugins/manifest-reference.md, "lspServers": "Each server config is a strict
 * object"; spec §9.1.2, the oracle table of §9.1.3). The messages are the oracle's (2.1.325): zod's
 * own wording, and for `command` and `extensionToLanguage` the two texts it adds. A type error, a
 * fraction or a key without its dot aborts the entry, so inside the `lspServers` union it is a bare
 * `Invalid input` at the key; a too-small number, an empty map, a command with spaces and an
 * unknown key stay at their own path.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/lsp_schema.rs`, docs/RUST.md).
 */
const call = coreModule('format/lsp-schema');

/** `stdio` (default) or `socket`: "Claude Code accepts `socket` but runs every server over stdio". */
export const LSP_TRANSPORTS: readonly ['stdio', 'socket'] = call('LSP_TRANSPORTS');
