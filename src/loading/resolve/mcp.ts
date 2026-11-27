import type { PluginIssue } from '../../format/issues.ts';
import type { PluginMcpServer } from '../../format/mcp-config.ts';
import type { CommandSpec } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HelperServer } from './headers-helper.ts';
import type { SubstitutionScope } from './scope.ts';

const call = coreModule('loading/resolve/mcp');

/**
 * A plugin's MCP server config resolved for this host (plugins/manifest-reference.md, "Where each
 * variable resolves"; mcp.md, "Plugin-provided MCP servers"). The kit never starts anything: it hands
 * the host the substituted config, the variables the process receives and, for a server with a
 * `headersHelper`, the environment that helper runs with.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/mcp.rs`, docs/RUST.md).
 */
export interface ResolvedMcpServer {
  /** The key as the plugin wrote it. */
  readonly name: string;
  /** What Claude Code registers it under: `plugin:<plugin>:<server>`. */
  readonly key: string;
  /**
   * The plugin it belongs to, by the id the host uses for it (`SubstitutionScope.pluginId`): what a
   * host keys a registration by. The same field on every resolved process (judgment, spec/judgments.md).
   */
  readonly pluginId: string;
  /** The config with `${...}` substituted. */
  readonly server: PluginMcpServer;
  /** Variables the process receives besides its `env`: the three path variables for stdio, none for a remote server. */
  readonly environment: Readonly<Record<string, string>>;
  /**
   * The server as `planHeadersHelper` takes it (with origin `plugin`): the registered key, the URL as
   * expanded, the environment variables the expansion substituted and the substituted command. The
   * plan gives the helper's environment and directory. Absent when the server has no helper.
   */
  readonly headersHelper?: HelperServer;
  /**
   * How a stdio server's process starts (`mcpStdioStart`): an exec of `command` and `args`, or under
   * `CLAUDE_CODE_SHELL_PREFIX` the prefix with the quoted command line (spec §8.2.1, J8.26). Absent
   * for a remote server.
   */
  readonly start?: CommandSpec;
  /**
   * A core package's server (Agent Plugins Extended §18.1.4): its stdio process runs in this working
   * directory (`cwd`, the plugin root by default), receives `PLUGIN_ROOT` and `PLUGIN_DATA` in
   * `environment` and none of the `CLAUDE_PLUGIN_*` variables: the host leaves those out of the base
   * environment it starts the process with (§18.1.3 item 1, J18.12).
   */
  readonly core?: { readonly cwd?: string };
}

export interface McpResolution {
  /** Undefined when the server does not start. */
  readonly resolved: ResolvedMcpServer | undefined;
  readonly issues: readonly PluginIssue[];
  /**
   * A remote server whose `url` is written empty: a placeholder for a connector the user configures
   * later. It is listed as `not configured`, not connected, and not an error (mcp.md, "not
   * configured"; spec §8.8 item 6). Its detail view reads `No URL configured for this server`.
   */
  readonly notConfigured?: true;
}

/** `plugin:<plugin>:<server>` (components.md, "Server names, tool names, and reloads"). */
export const mcpServerKey = (plugin: string, server: string): string =>
  call('mcpServerKey', plugin, server);

/**
 * Resolves one server of a plugin. stdio: `command`, `args` and `env` are substituted and the process
 * gets the three path variables. http, sse and ws: `url`, `headers` and `headersHelper` are
 * substituted, nothing is exported, and a `headersHelper` that references `${user_config.*}` keeps the
 * server from starting. So does a `url` that does not parse after substitution (plugins/
 * troubleshooting.md): `URL is unset or invalid` when it uses a `${user_config.*}` option with no value,
 * `has an invalid MCP url` when the plugin's own `url` is at fault (judgment: the docs name both causes,
 * not how they are told apart). An unset `${VAR}` with no default keeps it from starting too, as
 * `Missing environment variables: <names>`, in whichever field it is named.
 */
export const resolveMcpServer = (
  name: string,
  server: PluginMcpServer,
  scope: SubstitutionScope,
): Promise<McpResolution> => call.async('resolveMcpServer', name, server, scope);
