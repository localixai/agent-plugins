/**
 * The channel gate: whether channels may deliver messages on the account (settings-reference.md,
 * `channelsEnabled`; channels.md, "Enterprise controls").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/channels/channel_gate.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/channels/channel-gate');

/**
 * The account kinds the `channelsEnabled` default depends on (settings-reference.md, `channelsEnabled`;
 * channels.md, "Enterprise controls"). The host tells which one the session is signed in with.
 */
export const CHANNEL_ACCOUNTS: readonly ['claudeaiTeamEnterprise', 'claudeaiPersonal', 'console'] =
  call('CHANNEL_ACCOUNTS');
export type ChannelAccount = (typeof CHANNEL_ACCOUNTS)[number];

/** What the host tells the channel gate besides the layers: plain data, the kit reads no environment. */
export interface ChannelsContext {
  readonly account: ChannelAccount;
  /**
   * An administrator deploys managed settings on this machine or account. A managed layer among the
   * view's layers counts as managed settings too, so a host that cannot tell may pass `false`.
   */
  readonly hasManagedSettings: boolean;
}

/** What opened the gate. */
export type ChannelGateBy = 'noOrganization' | 'consoleDefault' | 'channelsEnabled';

export interface ChannelGate {
  readonly open: boolean;
  /** Why the gate is open; `undefined` while it is closed. */
  readonly by: ChannelGateBy | undefined;
  /**
   * "A startup warning tells the user to have an admin enable the setting" (channels.md, "Enable
   * channels for your organization"). The wording belongs to the host; this names the warning.
   */
  readonly startupWarning: 'adminMustEnable' | undefined;
  /**
   * Always `true`: a closed gate stops channel messages only. "The MCP server still connects and its
   * tools work, but channel messages won't arrive" (channels.md, "Enable channels for your
   * organization"). The kit starts no server; this tells the host not to disconnect one.
   */
  readonly serverConnects: true;
}

/**
 * The master switch: whether channels may deliver messages on this account. `channelsEnabled: false`
 * is "the same as unset" and the key counts from managed settings only (its Scope line, enforced by
 * the view), so a user cannot lift the block. A closed gate blocks every channel, the development
 * flag included (channels-reference.md, "Test during the research preview").
 */
export const channelGate = (view: PolicyView, context: ChannelsContext): ChannelGate =>
  call('channelGate', view, {
    account: context.account,
    hasManagedSettings: context.hasManagedSettings,
  });
