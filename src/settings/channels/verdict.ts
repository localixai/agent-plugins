/**
 * Whether a channel plugin a user passed for the session may register: the gate first, then the
 * rule of the flag the entry came from.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/channels/verdict.rs`, docs/RUST.md).
 */
import type { ChannelPluginRef } from '../../protocol/official.ts';
import type { ChannelAllowlist } from './allowlist.ts';
import type { ChannelGate } from './channel-gate.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/channels/verdict');

/** Why a channel plugin may or may not register. */
export const CHANNEL_REASONS: readonly [
  'organizationAllowlist',
  'defaultAllowlist',
  'developmentFlag',
  'channelsNotEnabled',
  'notOnOrganizationList',
  'notOnDefaultList',
  'developmentFlagNeedsInteractive',
] = call('CHANNEL_REASONS');
export type ChannelReason = (typeof CHANNEL_REASONS)[number];

export interface ChannelVerdict {
  readonly allowed: boolean;
  readonly reason: ChannelReason;
}

/** The two flags that carry a channel into a session. */
export const CHANNEL_ORIGINS: readonly ['channels', 'development'] = call('CHANNEL_ORIGINS');
export type ChannelOrigin = (typeof CHANNEL_ORIGINS)[number];

/** What each flag's entry tells the verdict, beyond the flag it came from. */
interface EntryByOrigin {
  /** `--channels plugin:<name>@<marketplace>`: faces the allowlist. */
  readonly channels: { readonly plugin: ChannelPluginRef };
  /**
   * `--dangerously-load-development-channels`, `plugin:<name>@<marketplace>` or a bare
   * `server:<name>`: skips the allowlist, so the verdict reads no plugin; it is set for the first form.
   */
  readonly development: { readonly interactive: boolean; readonly plugin?: ChannelPluginRef };
}

/**
 * One entry a user passed for the session, with the flag it came from. "The bypass is per-entry":
 * the development flag skips the list for its own entries only, and "doesn't extend the bypass to the
 * `--channels` entries" (channels-reference.md, "Test during the research preview"). Either way the
 * user opted in: "no channel runs until a user opts it in for the session" (channels.md, "Enterprise
 * controls").
 */
export type ChannelEntry = {
  readonly [O in ChannelOrigin]: { readonly origin: O } & EntryByOrigin[O];
}[ChannelOrigin];

/** The gate first (it blocks the development flag too), then the entry's own flag rule. */
export const channelVerdict = (
  gate: ChannelGate,
  allowlist: ChannelAllowlist,
  entry: ChannelEntry,
): ChannelVerdict => call('channelVerdict', gate, allowlist, entry);
