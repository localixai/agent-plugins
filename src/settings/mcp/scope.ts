/**
 * Where an MCP server comes from, highest precedence first (mcp.md, "Scope hierarchy and
 * precedence"): the organization's `managedMcpServers` "ranks above all of these", then local,
 * project and user scope, plugin-provided servers and claude.ai connectors.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/scope.rs`, docs/RUST.md).
 */
import type { PluginMcpServer } from '../../format/mcp-config.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/scope');

export const MCP_SCOPES: readonly ['managed', 'local', 'project', 'user', 'plugin', 'connector'] =
  call('MCP_SCOPES');
export type McpScope = (typeof MCP_SCOPES)[number];

/** A server some source defines, already read and with its `${VAR}` references expanded. */
export interface McpCandidate {
  readonly scope: McpScope;
  readonly name: string;
  readonly server: PluginMcpServer;
  /** `false` when the project's toggle turned it off (`applyToggles`); absent means on. */
  readonly enabled?: boolean;
}

export interface RankOptions {
  /**
   * The session is a local one in the Desktop app's Code tab: "the Code tab uses the `~/.claude.json`
   * definition" of a stdio server written in both user scope and `.mcp.json`.
   */
  readonly desktopCodeTab?: boolean;
}

/** A candidate a higher one of the same identity displaced: `/mcp` lists a hidden connector so. */
export interface HiddenMcpServer {
  readonly candidate: McpCandidate;
  readonly duplicateOf: McpCandidate;
}

export interface McpRanking {
  /** One server per identity, highest scope first; a disabled one is kept (hosts skip `enabled: false`). */
  readonly loaded: readonly McpCandidate[];
  readonly hidden: readonly HiddenMcpServer[];
}

/**
 * Loads each server once, from the highest-precedence source, using its whole entry (mcp.md, "Scope
 * hierarchy and precedence"). A server holds its endpoint against plugins and connectors only while
 * it is enabled ("one that points at the same URL or command as an enabled server above").
 */
export const rankMcpServers = (
  candidates: readonly McpCandidate[],
  options: RankOptions = {},
): McpRanking => call('rankMcpServers', candidates, options);
