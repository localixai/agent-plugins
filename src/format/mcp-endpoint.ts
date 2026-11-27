/**
 * What identifies the endpoint of an MCP server (mcp.md, "Scope hierarchy and precedence"): plugins
 * and connectors are matched with the servers above them by endpoint, "so one that points at the same
 * URL or command as an enabled server above counts as a duplicate".
 *
 * A URL: "Two URL spellings count as the same endpoint when they differ only in the letter case of the
 * scheme or host, the scheme's default port, such as `:443` on `https`, or a trailing slash. A
 * different path, query string, userinfo, or non-default port makes two servers." The URL parser
 * lowercases scheme and host and drops the default port; the trailing slash of a path is dropped here.
 *
 * Judgments (spec/judgments.md): a fragment is kept (the docs list the spellings that merge, and a
 * fragment is not among them); a URL that does not parse is its own endpoint by its text; a command is
 * compared with its arguments as a list, never joined into one string; the transport type is not
 * part of the key (the docs compare "the same URL or command").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_endpoint.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginMcpServer } from './mcp-config.ts';

const call = coreModule('format/mcp-endpoint');

/** A key equal for two servers exactly when Claude Code counts them as one endpoint. */
export const mcpEndpointKey = (server: PluginMcpServer): string => call('mcpEndpointKey', server);
