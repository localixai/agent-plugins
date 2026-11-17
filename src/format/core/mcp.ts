import { error, warning, type PluginIssue } from '../issues.ts';
import { documentMembers } from './document.ts';
import {
  CORE_TRANSPORTS,
  readCoreServer,
  type CoreMcpServer,
  type CoreTransport,
} from './mcp-server.ts';
import type { CoreVersion } from './versions.ts';

/**
 * A core package's `mcp.json` (core §7.2): the outer object disables MCP for the package when it
 * fails (core §7.2.2 item 2); a server entry that fails is skipped and reported, the others load
 * (item 3); a valid entry whose transport the host does not support is skipped with a warning, since
 * missing support is not an error (item 4, core §11.3 item 4).
 */

export interface CoreMcpOptions {
  /** The extension namespaces the host implements (core §8.1). */
  readonly implemented?: readonly string[];
  /** The transports the host supports; all three by default (core §7.2.1, "Transport support"). */
  readonly transports?: readonly CoreTransport[];
}

export interface CoreMcpResult {
  readonly servers: Readonly<Record<string, CoreMcpServer>>;
  /** MCP is disabled for the package: the outer object failed. */
  readonly disabled: boolean;
  readonly issues: readonly PluginIssue[];
}

const FILE = 'mcp.json';

/** Reads a parsed `mcp.json` against the version `plugin.json` targets. */
export function readCoreMcp(
  raw: unknown,
  manifestVersion: CoreVersion,
  options: CoreMcpOptions = {},
): CoreMcpResult {
  const outer = documentMembers(raw, 'mcp', 'mcpServers', manifestVersion);
  if ('problem' in outer) {
    return {
      servers: {},
      disabled: true,
      issues: [error(FILE, `MCP is disabled: ${outer.problem}`)],
    };
  }
  const transports = options.transports ?? CORE_TRANSPORTS;
  const servers: Record<string, CoreMcpServer> = {};
  const issues: PluginIssue[] = [];
  for (const [name, entry] of Object.entries(outer.members)) {
    const path = `mcpServers.${name}`;
    const read = readCoreServer(entry, options.implemented ?? []);
    if ('problem' in read) {
      issues.push(error(path, `Server "${name}" is skipped: ${read.problem}`));
    } else if (transports.includes(read.server.type)) {
      servers[name] = read.server;
    } else {
      issues.push(
        warning(
          path,
          `Server "${name}" is skipped: this client does not support "${read.server.type}"`,
        ),
      );
    }
  }
  return { servers, disabled: false, issues };
}
