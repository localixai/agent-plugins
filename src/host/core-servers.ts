import type { CoreMcpServer, CoreStdioServer } from '../format/core/mcp-server.ts';
import type { CorePaths } from '../format/core/placeholders.ts';
import { coreServerProcess } from '../format/core/subprocess.ts';
import { error, type PluginIssue } from '../format/issues.ts';
import { pluginNameOf } from '../format/marketplace/ids.ts';
import type { CorePackage } from '../loading/core/index.ts';
import { mcpServerKey, type ResolvedMcpServer } from '../loading/index.ts';
import { mcpStdioStart } from '../loading/resolve/shell-prefix.ts';
import type { Environment, Locations } from '../ports/index.ts';
import type { ActivePlugin } from './state.ts';

/**
 * A core package's MCP servers in the session's one tool namespace (Agent Plugins Extended §18.1.4,
 * J18.10, J18.11): registered under `plugin:<name>:<server>`, so their tools are
 * `mcp__plugin_<name>_<server>__<tool>` for the model, the permission rules, the hook matchers and
 * the policy lists. The loader validated each entry by core §7.2; here a valid one becomes a Claude
 * Code plugin server of the same transport with no optional field set: `stdio` as is, with its
 * `./` command resolved, `args`, `env` and `cwd` expanded (core §9.2) and the `cwd` contained;
 * `streamable-http` as `http`; `sse` as `sse`; `url` and `headers` never expanded. The process gets
 * `PLUGIN_ROOT`, `PLUGIN_DATA` and `CLAUDE_PROJECT_DIR` (§18.1.3, J18.12), no `CLAUDE_PLUGIN_*`.
 */

export interface CorePlace {
  readonly locations: Locations;
  readonly environment: Environment;
  readonly signal?: AbortSignal;
}

/** The two directories of core §9.1: the install's root and its data directory (J18.13). */
export async function corePathsOf(plugin: ActivePlugin, place: CorePlace): Promise<CorePaths> {
  return {
    root: place.locations.pluginRoot(plugin.id),
    data: await place.locations.pluginData(plugin.id, place.signal),
  };
}

const RESERVED: ReadonlySet<string> = new Set(['PLUGIN_ROOT', 'PLUGIN_DATA']);

function stdioServer(
  base: Omit<ResolvedMcpServer, 'server' | 'environment'>,
  server: CoreStdioServer,
  paths: CorePaths,
  place: CorePlace,
): { readonly resolved?: ResolvedMcpServer; readonly issue?: PluginIssue } {
  const made = coreServerProcess(server, { paths });
  if ('problem' in made) {
    return {
      issue: error(`mcpServers.${base.name}`, `Server "${base.name}" is skipped: ${made.problem}`),
    };
  }
  const launch = made.process;
  // `env` is the server's own, expanded; the two reserved variables come in `environment`.
  const env = Object.fromEntries(
    Object.entries(launch.env).filter(([name]) => !RESERVED.has(name)),
  );
  return {
    resolved: {
      ...base,
      server: { type: 'stdio', command: launch.command, args: [...launch.args], env },
      environment: {
        PLUGIN_ROOT: paths.root,
        PLUGIN_DATA: paths.data,
        CLAUDE_PROJECT_DIR: place.locations.projectDir(),
      },
      start: mcpStdioStart({ command: launch.command, args: [...launch.args] }, place.environment),
      core: { cwd: launch.cwd },
    },
  };
}

/** The Claude Code transport each remote core transport runs as (§18.1.4 item 3). */
const REMOTE_TYPE = { 'streamable-http': 'http', sse: 'sse' } as const;

function serverOf(
  plugin: ActivePlugin,
  name: string,
  server: CoreMcpServer,
  context: { readonly paths: CorePaths; readonly place: CorePlace },
): { readonly resolved?: ResolvedMcpServer; readonly issue?: PluginIssue } {
  const base = {
    name,
    key: mcpServerKey(pluginNameOf(plugin.id), name),
    pluginId: plugin.id,
  };
  if ('command' in server) return stdioServer(base, server, context.paths, context.place);
  const { url, headers } = server;
  return {
    resolved: {
      ...base,
      server: {
        type: REMOTE_TYPE[server.type],
        url,
        ...(headers === undefined ? {} : { headers: { ...headers } }),
      },
      environment: {},
      core: {},
    },
  };
}

/** The servers of one core package, in the order `mcp.json` names them. */
export async function coreServers(
  plugin: ActivePlugin,
  core: CorePackage,
  place: CorePlace,
): Promise<{
  readonly resolved: readonly ResolvedMcpServer[];
  readonly issues: readonly PluginIssue[];
}> {
  const entries = Object.entries(core.mcpServers);
  if (entries.length === 0) return { resolved: [], issues: [] };
  const paths = await corePathsOf(plugin, place);
  const made = entries.map(([name, server]) => serverOf(plugin, name, server, { paths, place }));
  return {
    resolved: made.flatMap((one) => (one.resolved === undefined ? [] : [one.resolved])),
    issues: made.flatMap((one) => (one.issue === undefined ? [] : [one.issue])),
  };
}
