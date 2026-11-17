import { warning } from './issues.ts';
import { mcpServerNameReserved } from './host-errors.ts';
import type { McpConfigRead } from './mcp-config.ts';

/**
 * The server names Claude Code reserves (mcp.md; spec §8.5 item 5): a server of the user's own
 * configuration under one of them is skipped. A plugin's server key is not checked, because the
 * server registers under its scoped name (J8.12), so plugin readers never call this.
 */
export const RESERVED_MCP_SERVER_NAMES = [
  'workspace',
  'claude-in-chrome',
  'computer-use',
  'Claude Preview',
  'Claude Browser',
] as const;

const RESERVED: ReadonlySet<string> = new Set(RESERVED_MCP_SERVER_NAMES);

/**
 * The servers of a user configuration (`~/.claude.json`, a project's `.mcp.json`) without those
 * under a reserved name, each reported as `"<name>" is a reserved MCP server name and was not
 * loaded` (warning). The match is exact, as the names are written.
 */
export function withoutReservedServers(read: McpConfigRead, origin: string): McpConfigRead {
  const names = Object.keys(read.servers);
  const reserved = names.filter((name) => RESERVED.has(name));
  if (reserved.length === 0) return read;
  const servers = Object.fromEntries(
    Object.entries(read.servers).filter(([name]) => !RESERVED.has(name)),
  );
  return {
    ...read,
    servers,
    issues: [
      ...read.issues,
      ...reserved.map((name) => warning(origin, mcpServerNameReserved(name))),
    ],
  };
}
