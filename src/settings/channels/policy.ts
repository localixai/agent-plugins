/** Implemented by the Rust core (`crates/agent-plugins/src/settings/channels/policy.rs`, docs/RUST.md). */
import type { PolicyView } from '../view.ts';
import type { ChannelAllowlist, ChannelAllowlistContext } from './allowlist.ts';
import type { ChannelGate } from './channel-gate.ts';
import type { ChannelEntry, ChannelVerdict } from './verdict.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/channels/policy');

/** What the host tells the channel keys besides the layers. */
export type ChannelPolicyContext = ChannelAllowlistContext;

/** The channel keys of the settings as one object over a view (docs/API.md, "Settings"). */
export interface ChannelSettingsPolicy {
  /** `channelsEnabled`: whether channels may deliver messages on this account. */
  gate(): ChannelGate;
  /** `allowedChannelPlugins` over the host's default list. */
  allowlist(): ChannelAllowlist;
  /** Whether an entry the user passed for the session may register as a channel. */
  allowed(entry: ChannelEntry): ChannelVerdict;
}

export const createChannelSettingsPolicy = (
  view: PolicyView,
  context: ChannelPolicyContext,
): ChannelSettingsPolicy =>
  call('createChannelSettingsPolicy', view, {
    account: context.account,
    hasManagedSettings: context.hasManagedSettings,
    defaultChannelAllowlist: context.defaultChannelAllowlist,
  });
