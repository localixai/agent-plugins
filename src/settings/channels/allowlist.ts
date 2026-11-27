/**
 * The channel allowlist in force (channels.md, "Enterprise controls"; managed-settings.md,
 * "Restriction allowlists").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/channels/allowlist.rs`, docs/RUST.md).
 */
import type { ChannelPluginRef } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';
import type { ChannelsContext } from './channel-gate.ts';

const call = coreModule('settings/channels/allowlist');

/** What the host tells the allowlist besides the layers. */
export interface ChannelAllowlistContext extends ChannelsContext {
  /**
   * The Anthropic-maintained default allowlist. Its content is not in the docs ("the channel plugins
   * in claude-plugins-official are the default approved set", channels.md), so the host supplies it.
   */
  readonly defaultChannelAllowlist: readonly ChannelPluginRef[];
}

export interface ChannelAllowlist {
  /** `organization` when `allowedChannelPlugins` replaced the default, `default` otherwise. */
  readonly source: 'organization' | 'default';
  readonly entries: readonly ChannelPluginRef[];
}

/**
 * The allowlist in force. `allowedChannelPlugins` "replaces the Anthropic-maintained list when set",
 * an empty list included, and is taken whole from the highest managed source (managed-settings.md,
 * "Restriction allowlists"; the view's merge). Entries are `{ plugin, marketplace }`: the string
 * `plugin@marketplace` was read as the equivalent object when the layer was read. An account without
 * an organization skips the key (channels.md, "Enterprise controls"; spec/judgments.md).
 */
export const effectiveChannelAllowlist = (
  view: PolicyView,
  context: ChannelAllowlistContext,
): ChannelAllowlist =>
  call('effectiveChannelAllowlist', view, {
    account: context.account,
    hasManagedSettings: context.hasManagedSettings,
    defaultChannelAllowlist: context.defaultChannelAllowlist,
  });
