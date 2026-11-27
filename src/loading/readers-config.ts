import { coreModule } from '../wasm/bridge.ts';
import type { ComponentRead, ComponentValues, ReaderContext } from './reader-types.ts';

/**
 * The component readers of the JSON configuration a plugin carries: hooks, LSP servers, monitors,
 * MCP servers and default settings. A file is parsed first and read by its kind's format reader; an
 * inline manifest value is read as is.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/readers_config.rs`, docs/RUST.md).
 */

const call = coreModule('loading/readers-config');

export const readHooks = (
  context: ReaderContext,
): Promise<ComponentRead<ComponentValues['hooks']>> => call.async('readHooks', context);

export const readLspServers = (
  context: ReaderContext,
): Promise<ComponentRead<ComponentValues['lspServers']>> => call.async('readLspServers', context);

export const readMonitorList = (
  context: ReaderContext,
): Promise<ComponentRead<ComponentValues['monitors']>> => call.async('readMonitorList', context);

export const readMcpServers = (
  context: ReaderContext,
): Promise<ComponentRead<ComponentValues['mcpServers']>> => call.async('readMcpServers', context);

/**
 * `settings.json` over the manifest's `settings` key: "when both exist and `settings.json` sets at
 * least one supported key, `settings.json` applies and the manifest's `settings` is ignored"
 * (components.md, "Default settings").
 */
export const readSettings = (
  context: ReaderContext,
): Promise<ComponentRead<ComponentValues['settings']>> => call.async('readSettings', context);
