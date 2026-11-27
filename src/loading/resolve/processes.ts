/**
 * The long-running processes a plugin declares, resolved for this host: MCP servers, language
 * servers and monitors. The kit never spawns any of them: the host starts each and reads the issues
 * for the ones that must not start (a `${user_config.*}` in a shell field, an unparseable URL).
 * An entry that does not start is left out and its issue is reported; the others are unaffected.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/processes.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { Plugin } from '../plugin-model.ts';
import type { ResolvedLspServer } from './lsp.ts';
import type { ResolvedMcpServer } from './mcp.ts';
import type { ResolvedMonitor } from './monitor.ts';
import type { SubstitutionScope } from './scope.ts';

const call = coreModule('loading/resolve/processes');

export interface ResolvedProcesses {
  /** By the name the plugin wrote; each carries the `plugin:<plugin>:<server>` key `/mcp` shows. */
  readonly mcpServers: Readonly<Record<string, ResolvedMcpServer>>;
  /** By the name the plugin wrote; each carries its `pluginId`, to key it by across plugins. */
  readonly lspServers: Readonly<Record<string, ResolvedLspServer>>;
  /** In declaration order; `monitorsToStart` decides when each starts. */
  readonly monitors: readonly ResolvedMonitor[];
  readonly issues: readonly PluginIssue[];
}

/** One kind of process: what starts, by name for servers, and why the others do not. */
export interface ResolvedKind<R> {
  readonly resolved: R;
  readonly issues: readonly PluginIssue[];
}

/**
 * The plugin's MCP servers. A bundled server resolves with its bundle's own settings; a server a channel binds to resolves with the channel's options
 * (manifest-reference.md, "Channels": "Saved values substitute into `${user_config.KEY}`
 * references in the server's `env`").
 */
export const resolveMcpServers = (
  plugin: Pick<Plugin, 'mcpServers' | 'mcpBundles' | 'manifest'>,
  scope: SubstitutionScope,
): Promise<ResolvedKind<Record<string, ResolvedMcpServer>>> =>
  call.async(
    'resolveMcpServers',
    { mcpServers: plugin.mcpServers, mcpBundles: plugin.mcpBundles, manifest: plugin.manifest },
    scope,
  );

export const resolveLspServers = (
  plugin: Pick<Plugin, 'lspServers'>,
  scope: SubstitutionScope,
): Promise<ResolvedKind<Record<string, ResolvedLspServer>>> =>
  call.async('resolveLspServers', { lspServers: plugin.lspServers }, scope);

export const resolveMonitors = (
  plugin: Pick<Plugin, 'monitors'>,
  scope: SubstitutionScope,
): Promise<ResolvedKind<ResolvedMonitor[]>> =>
  call.async('resolveMonitors', { monitors: plugin.monitors }, scope);

export const resolveProcesses = (
  plugin: Pick<Plugin, 'mcpServers' | 'mcpBundles' | 'lspServers' | 'monitors' | 'manifest'>,
  scope: SubstitutionScope,
): Promise<ResolvedProcesses> =>
  call.async(
    'resolveProcesses',
    {
      mcpServers: plugin.mcpServers,
      mcpBundles: plugin.mcpBundles,
      lspServers: plugin.lspServers,
      monitors: plugin.monitors,
      manifest: plugin.manifest,
    },
    scope,
  );
