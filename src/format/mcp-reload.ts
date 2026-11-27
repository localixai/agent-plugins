/**
 * How a plugin's MCP servers follow a reload (mcp.md, "Plugin-provided MCP servers", "Automatic
 * lifecycle"; plugins/components.md, "Server names, tool names, and reloads"): "when the user runs
 * `/reload-plugins` ... a server whose configuration is unchanged keeps its connection. A server whose
 * configuration changed reconnects, and one you removed disconnects"; the same holds when the session
 * moves with `/cd` to a directory whose settings enable other plugins. Pure over the resolved configs;
 * connecting is the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_reload.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginMcpServer } from './mcp-config.ts';

const call = coreModule('format/mcp-reload');

export interface McpReloadPlan {
  /** In both lists with the same configuration: the live connection stays. */
  readonly keep: readonly string[];
  /** In both with a changed configuration. */
  readonly reconnect: readonly string[];
  /** Only in the new list: a plugin was enabled, or a server added. */
  readonly connect: readonly string[];
  /** Only in the old list: a plugin was disabled, or a server removed. */
  readonly disconnect: readonly string[];
}

/**
 * What a reload does to each server, by the name the host registers it under. Names appear in the
 * lists in the order of the new map, then of the old one for `disconnect`.
 */
export const planMcpReload = (
  previous: Readonly<Record<string, PluginMcpServer>>,
  next: Readonly<Record<string, PluginMcpServer>>,
): McpReloadPlan => call('planMcpReload', previous, next);

/** "A connection to that server failed less than 15 minutes earlier." */
export const MCP_FAILURE_SKIP_MS: number = call('MCP_FAILURE_SKIP_MS');

/** errors.md quotes it as the status of a server that was not started. */
export const SKIPPING_CONNECTION: string = call('SKIPPING_CONNECTION');

export interface McpConnectionFailure {
  readonly atMs: number;
  /** The configuration the failed connection used. */
  readonly server: PluginMcpServer;
}

/**
 * Whether Claude Code skips connecting a plugin's server: it is a stdio server whose connection failed
 * "less than 15 minutes earlier", with the configuration it has now ("or edit the plugin config to
 * retry now"). A remote server is not cached this way. Judgment (spec/judgments.md): a failure exactly
 * 15 minutes old is old enough, and one dated in the future does not count.
 */
export const skipsStdioConnection = (
  server: PluginMcpServer,
  failure: McpConnectionFailure | undefined,
  nowMs: number,
): boolean => call('skipsStdioConnection', server, failure, nowMs);
