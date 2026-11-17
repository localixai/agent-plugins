import { readCoreServer } from '../../format/core/mcp-server.ts';
import {
  mcpTransportOf,
  type McpTransport,
  type PluginMcpServer,
} from '../../format/mcp-config.ts';
import { notConverted } from '../notes.ts';
import {
  bridgedEnvironment,
  holdsOtherReference,
  holdsReference,
  toCorePlaceholders,
} from '../placeholders.ts';
import type { ConversionNote } from '../types.ts';

/**
 * One core `mcp.json` from a Claude Code plugin's servers (Agent Plugins Extended §18.4.3 item 3):
 * `.mcp.json`, the manifest's files and inline maps, already merged by the later-name-wins rule of
 * §2.9 when the plugin loaded. `stdio` (or no `type`) stays `stdio`, `http` is `streamable-http`,
 * `sse` stays; `${CLAUDE_PLUGIN_ROOT}/path` as `command` becomes `./path`, a bare name stays;
 * `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}` in `args` and `env` become the core's names, and
 * a stdio server gets `CLAUDE_PLUGIN_ROOT` and `CLAUDE_PLUGIN_DATA` in `env` (J18.18). A `ws` server,
 * a bundle, a field the core lacks (`headersHelper`, `oauth`, `timeout`, `alwaysLoad`), any other
 * `${…}` reference (`${VAR}`, `${user_config.*}`) and an entry the core schema refuses (an `http` URL
 * off loopback, user information, a fragment) are not converted, each reported.
 */

export const CORE_MCP = 'mcp.json';
const SOURCE = '.mcp.json';

type Raw = Readonly<Record<string, unknown>>;
type Mapped = { readonly raw: Raw } | { readonly problem: string };

const FIELDS: { readonly [T in McpTransport]: ReadonlySet<string> } = {
  stdio: new Set(['type', 'command', 'args', 'env']),
  http: new Set(['type', 'url', 'headers']),
  sse: new Set(['type', 'url', 'headers']),
  ws: new Set(),
};

const ROOT_PREFIX = '${CLAUDE_PLUGIN_ROOT}/';

function coreCommand(command: string): string | undefined {
  if (command.startsWith(ROOT_PREFIX)) return `./${command.slice(ROOT_PREFIX.length)}`;
  return /^[^\s/\\$]+$/.test(command) ? command : undefined;
}

const strings = (value: unknown): readonly string[] =>
  Array.isArray(value) ? value.filter((one): one is string => typeof one === 'string') : [];

const stringMap = (value: unknown): Readonly<Record<string, string>> =>
  typeof value === 'object' && value !== null
    ? Object.fromEntries(
        Object.entries(value).filter(
          (entry): entry is [string, string] => typeof entry[1] === 'string',
        ),
      )
    : {};

function stdio(server: Raw): Mapped {
  const command = coreCommand(typeof server.command === 'string' ? server.command : '');
  if (command === undefined) {
    return {
      problem: `"command" ${JSON.stringify(server.command)} is neither \${CLAUDE_PLUGIN_ROOT}/<path> nor a bare name`,
    };
  }
  const args = strings(server.args);
  const env = stringMap(server.env);
  if ([...args, ...Object.values(env)].some((text) => holdsOtherReference(text, 'claude-code'))) {
    return { problem: 'its args or env hold a "${…}" reference the core does not expand' };
  }
  return {
    raw: {
      type: 'stdio',
      command,
      ...(args.length === 0 ? {} : { args: args.map(toCorePlaceholders) }),
      env: bridgedEnvironment(env, 'claude-code', toCorePlaceholders),
    },
  };
}

const remote =
  (type: 'streamable-http' | 'sse') =>
  (server: Raw): Mapped => {
    const headers = server.headers === undefined ? undefined : stringMap(server.headers);
    const texts = [String(server.url), ...Object.values(headers ?? {})];
    if (texts.some(holdsReference)) {
      return {
        problem: 'its url or headers hold a "${…}" reference, which the core keeps literally',
      };
    }
    return { raw: { type, url: server.url, ...(headers === undefined ? {} : { headers }) } };
  };

/** One mapping per Claude Code transport (K1). */
const MAPPINGS: { readonly [T in McpTransport]: (server: Raw) => Mapped } = {
  stdio,
  http: remote('streamable-http'),
  sse: remote('sse'),
  ws: () => ({ problem: 'the ws transport has no core counterpart' }),
};

function mapped(server: PluginMcpServer, bundled: boolean): Mapped {
  if (bundled) return { problem: 'an MCP bundle has no core counterpart' };
  const transport = mcpTransportOf(server);
  const result = MAPPINGS[transport](Object.fromEntries(Object.entries(server)));
  if (!('raw' in result)) return result;
  const extra = Object.keys(server).filter((key) => !FIELDS[transport].has(key));
  if (extra.length > 0) {
    return { problem: `${extra.map((key) => `"${key}"`).join(', ')} has no core counterpart` };
  }
  const read = readCoreServer(result.raw, []);
  return 'problem' in read ? { problem: `the core refuses it: ${read.problem}` } : result;
}

export interface CoreMcpPart {
  readonly servers: Readonly<Record<string, Raw>>;
  readonly notes: readonly ConversionNote[];
}

export function coreMcpOf(
  servers: Readonly<Record<string, PluginMcpServer>>,
  bundles: ReadonlySet<string>,
): CoreMcpPart {
  const results = Object.entries(servers).map(([name, server]) => ({
    name,
    result: mapped(server, bundles.has(name)),
  }));
  return {
    servers: Object.fromEntries(
      results.flatMap(({ name, result }) => ('raw' in result ? [[name, result.raw]] : [])),
    ),
    notes: results.flatMap(({ name, result }) =>
      'problem' in result
        ? [
            notConverted(
              SOURCE,
              `mcpServers.${name}`,
              `server "${name}" not converted: ${result.problem}`,
            ),
          ]
        : [],
    ),
  };
}
