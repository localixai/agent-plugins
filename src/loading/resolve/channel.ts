/**
 * A channel of a plugin as the dialog and the host need it (plugins/manifest-reference.md,
 * "Channels"): the MCP server it binds to, the name shown in the configuration dialog title, the
 * options to prompt for, and the server's config with the saved values substituted into `env`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/channel.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { PluginChannel } from '../../format/manifest-schema.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ResolvedMcpServer } from './mcp.ts';
import type { Plugin } from '../plugin-model.ts';
import type { SubstitutionScope } from './scope.ts';

const call = coreModule('loading/resolve/channel');

export interface ResolvedChannel {
  /** The key of the plugin's MCP server the channel binds to. */
  readonly server: string;
  /** "Name shown in the configuration dialog title. Defaults to the server name". */
  readonly displayName: string;
  /** The options to prompt for, in the shape of the top-level `userConfig`. */
  readonly userConfig: NonNullable<PluginChannel['userConfig']>;
  /** The bound server resolved with the channel's options; undefined when it does not start. */
  readonly mcp: ResolvedMcpServer | undefined;
}

export interface ChannelResolution {
  /** Undefined when the channel is bound to no server of the plugin. */
  readonly resolved: ResolvedChannel | undefined;
  readonly issues: readonly PluginIssue[];
}

/**
 * Resolves one channel against the plugin's servers. "Saved values substitute into
 * `${user_config.KEY}` references in the server's `env`": the channel's options join the plugin's
 * for this server, so `${user_config.bot_token}` in `env` takes the channel's `bot_token` (a
 * channel option wins over a top-level one of the same key; judgment, spec/judgments.md). A bundled
 * server reads its bundle's settings (`Plugin.mcpBundles`), as it does in `resolveMcpServers`.
 */
export const resolveChannel = (
  channel: PluginChannel,
  plugin: Pick<Plugin, 'mcpServers' | 'mcpBundles'>,
  scope: SubstitutionScope,
): Promise<ChannelResolution> =>
  call.async(
    'resolveChannel',
    channel,
    { mcpServers: plugin.mcpServers, mcpBundles: plugin.mcpBundles },
    scope,
  );
