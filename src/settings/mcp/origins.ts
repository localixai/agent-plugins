/**
 * The origins of an MCP server as the allow and deny lists read them, and the rule of each.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/origins.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/origins');

/**
 * Where a server comes from, as far as the lists are concerned (managed-mcp.md, "How a server is
 * evaluated" and "Servers that skip the allowlist check"). `configured` is every server a user, a
 * plugin or claude.ai adds, an agent's frontmatter declares or `--mcp-config` passes: all three checks.
 */
export const MCP_SERVER_ORIGINS: readonly [
  'configured',
  'managedSettings',
  'managedFile',
  'builtIn',
  'claudeTagSlack',
  'inProcess',
] = call('MCP_SERVER_ORIGINS');
export type McpServerOrigin = (typeof MCP_SERVER_ORIGINS)[number];
