import type {
  CoreMcpServer,
  CoreRemoteServer,
  CoreStdioServer,
} from '../../format/core/mcp-server.ts';
import { notConverted } from '../notes.ts';
import {
  bridgedEnvironment,
  holdsOtherReference,
  toClaudeCodePlaceholders,
} from '../placeholders.ts';
import type { ConversionNote } from '../types.ts';

/**
 * `.mcp.json` from a core `mcp.json` (Agent Plugins Extended §18.4.2 item 3, the table of §18.3.3):
 * `stdio` stays, `streamable-http` is `http`, `sse` stays; a `./` command is
 * `${CLAUDE_PLUGIN_ROOT}/…`; `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` become Claude Code's names, and a
 * stdio server gets `PLUGIN_ROOT` and `PLUGIN_DATA` in `env` (J18.18). `cwd` and `extensions` are
 * left out and reported; a server with a `${` left in `args`, `env`, `url` or `headers`, which Claude
 * Code would expand as a variable (§8.4.2) with no escape, is not written. Entries the core skipped
 * are not in the package, so they are not written either.
 */

export const CLAUDE_MCP = '.mcp.json';
const SOURCE = 'mcp.json';

type Written = Readonly<Record<string, unknown>>;
interface ServerPart {
  readonly server?: Written;
  readonly notes: readonly ConversionNote[];
}

const at = (name: string, field?: string): string =>
  field === undefined ? `mcpServers.${name}` : `mcpServers.${name}.${field}`;

function leftReferences(name: string, values: Readonly<Record<string, readonly string[]>>) {
  const fields = Object.entries(values)
    .filter(([, texts]) => texts.some((text) => holdsOtherReference(text, 'core')))
    .map(([field]) => field);
  return fields.length === 0
    ? undefined
    : notConverted(
        SOURCE,
        at(name),
        `server "${name}" not converted: ${fields.join(', ')} hold "\${" text, which Claude Code would expand as a variable`,
      );
}

function stdioServer(name: string, server: CoreStdioServer): ServerPart {
  const refused = leftReferences(name, {
    args: server.args ?? [],
    env: Object.values(server.env ?? {}),
  });
  if (refused !== undefined) return { notes: [refused] };
  const args = (server.args ?? []).map(toClaudeCodePlaceholders);
  const env = bridgedEnvironment(server.env, 'core', toClaudeCodePlaceholders);
  const notes =
    server.cwd === undefined
      ? []
      : [
          notConverted(
            SOURCE,
            at(name, 'cwd'),
            `"cwd" has no Claude Code counterpart: server "${name}" starts in Claude Code's working directory, not "${server.cwd}"`,
          ),
        ];
  const command = server.command.startsWith('./')
    ? `\${CLAUDE_PLUGIN_ROOT}/${server.command.slice(2)}`
    : server.command;
  return {
    server: { type: 'stdio', command, ...(args.length === 0 ? {} : { args }), env },
    notes,
  };
}

/** The Claude Code transport of each remote core transport (§18.3.3). */
const REMOTE_TYPE = { 'streamable-http': 'http', sse: 'sse' } as const;

function remoteServer(name: string, server: CoreRemoteServer): ServerPart {
  const headers = server.headers ?? {};
  const refused = leftReferences(name, { url: [server.url], headers: Object.values(headers) });
  if (refused !== undefined) return { notes: [refused] };
  const type = REMOTE_TYPE[server.type];
  return {
    server: { type, url: server.url, ...(server.headers === undefined ? {} : { headers }) },
    notes: [],
  };
}

const extensionNotes = (name: string, server: CoreMcpServer): ConversionNote[] =>
  Object.keys(server.extensions ?? {}).map((namespace) =>
    notConverted(
      SOURCE,
      at(name, `extensions.${namespace}`),
      'client extension data has no Claude Code counterpart',
    ),
  );

export interface McpPart {
  readonly servers: Readonly<Record<string, Written>>;
  readonly notes: readonly ConversionNote[];
}

export function claudeCodeMcp(servers: Readonly<Record<string, CoreMcpServer>>): McpPart {
  const parts = Object.entries(servers).map(([name, server]) => {
    const part = 'command' in server ? stdioServer(name, server) : remoteServer(name, server);
    return { name, part, notes: [...part.notes, ...extensionNotes(name, server)] };
  });
  return {
    servers: Object.fromEntries(
      parts.flatMap(({ name, part }) => (part.server === undefined ? [] : [[name, part.server]])),
    ),
    notes: parts.flatMap((one) => one.notes),
  };
}
