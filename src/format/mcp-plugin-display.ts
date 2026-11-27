/**
 * What `claude mcp get` and `claude mcp list` show of a plugin's servers (mcp.md, "Plugin-provided MCP
 * servers"): "`claude mcp get` hides the values configured for a plugin's stdio or WebSocket server,
 * because they can carry credentials". Pure data: the lines, not the terminal.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_plugin_display.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginMcpServer } from './mcp-config.ts';

const call = coreModule('format/mcp-plugin-display');

export interface RedactedPluginServer {
  /** The lines `claude mcp get` prints for the command, the URL, the arguments and the variables. */
  readonly get: readonly string[];
  /** What `claude mcp list` shows in place of the command or URL; absent where the docs name none. */
  readonly list?: string;
}

/**
 * The redacted view of a plugin's stdio or WebSocket server, or `undefined` for an http or sse server,
 * whose values the docs do not say are hidden.
 */
export const redactPluginServer = (server: PluginMcpServer): RedactedPluginServer | undefined =>
  call('redactPluginServer', server);
