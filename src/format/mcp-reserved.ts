import { coreModule } from '../wasm/bridge.ts';
import type { McpConfigRead } from './mcp-config.ts';

/**
 * The server names Claude Code reserves (mcp.md; spec §8.5 item 5): a server of the user's own
 * configuration under one of them is skipped. A plugin's server key is not checked, because the
 * server registers under its scoped name (J8.12), so plugin readers never call this.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_reserved.rs`, docs/RUST.md).
 */
const call = coreModule('format/mcp-reserved');

export const RESERVED_MCP_SERVER_NAMES: readonly [
  'workspace',
  'claude-in-chrome',
  'computer-use',
  'Claude Preview',
  'Claude Browser',
] = call('RESERVED_MCP_SERVER_NAMES');

/**
 * The servers of a user configuration (`~/.claude.json`, a project's `.mcp.json`) without those
 * under a reserved name, each reported as `"<name>" is a reserved MCP server name and was not
 * loaded` (warning). The match is exact, as the names are written.
 */
export const withoutReservedServers = (read: McpConfigRead, origin: string): McpConfigRead =>
  call('withoutReservedServers', read, origin);
