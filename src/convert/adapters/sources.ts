import { coreModule } from '../../wasm/bridge.ts';

/**
 * The two hook adapters a conversion adds (Agent Plugins Extended §18.4.4), as the text of Node.js
 * ES modules: self-contained, since they run where the kit is not installed, and the same bytes for
 * every conversion (§18.4.1 item 5). The runtime they need is `node` on `PATH`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/adapters/sources.rs`,
 * docs/RUST.md).
 */
const call = coreModule('convert/adapters/sources');

export const ADAPTER_RUNTIME: string = call('ADAPTER_RUNTIME');

/**
 * Runs a Claude Code `command` hook under a core client: `run-claude-code-handler.mjs --event <event>
 * [--profile <name>] [--timeout <seconds>] [--shell bash|powershell] [--exec] -- <command> [args...]`.
 */
export const RUN_CLAUDE_CODE_HANDLER_SOURCE: string = call('RUN_CLAUDE_CODE_HANDLER_SOURCE');
