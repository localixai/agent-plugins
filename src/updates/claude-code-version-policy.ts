import type { PolicyLayer } from '../settings/layers.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The version policy of Claude Code itself (settings-reference.md, "Updates and versioning":
 * `autoUpdatesChannel`, `minimumVersion`, `requiredMinimumVersion`, `requiredMaximumVersion`;
 * setup.md, "Configure release channel" and "Pin a minimum version"). Pure over the settings layers
 * and the versions the host knows: the kit decides what an update may install and whether a start is
 * refused; the host installs and exits.
 *
 * The four keys are read here, not through `settings/key-spec.ts`: that table is closed over the
 * plugin, hook, skill, workflow and channel keys the settings requirements name, and its tests count
 * them (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/updates/claude_code_version_policy.rs`,
 * docs/RUST.md).
 */

const call = coreModule('updates/claude-code-version-policy');

export const CHANNELS: readonly ['latest', 'stable'] = call('CHANNELS');
export type Channel = (typeof CHANNELS)[number];

export interface VersionPolicy {
  /** `autoUpdatesChannel`; unset (or a value the docs do not list) follows `latest`. */
  readonly channel: Channel;
  /** The highest valid `minimumVersion` of the files, normalized. */
  readonly minimumVersion?: string;
  /** Managed settings only. */
  readonly requiredMinimumVersion?: string;
  /** Managed settings only. */
  readonly requiredMaximumVersion?: string;
}

/**
 * Judgments (spec/judgments.md): the channel is the highest-precedence valid one; the minimum is the
 * highest valid `minimumVersion` of the files, so a managed floor "user and project settings can't
 * lower" holds while a user may raise it.
 */
export const readVersionPolicy = (layers: readonly PolicyLayer[]): VersionPolicy =>
  call('readVersionPolicy', layers);

/** What the host knows of the releases: the installed version and what each channel serves. */
export interface KnownVersions {
  readonly installed: string;
  readonly latest: string;
  readonly stable: string;
}

export type SkipReason =
  | 'channel-version-unknown'
  | 'already-current'
  | 'above-required-maximum'
  | 'below-required-minimum'
  | 'below-minimum-version';

export type UpdatePick =
  | {
      readonly outcome: 'install';
      readonly version: string;
      readonly from: Channel;
      /** The channel's version is older than the installed one (moving to `stable` from a newer `latest`). */
      readonly downgrade: boolean;
    }
  | { readonly outcome: 'skip'; readonly reason: SkipReason };

/**
 * What a background auto-update or `claude update` installs: the channel's version, unless it is
 * already installed or the policy rules it out. Nothing else is offered, so a refused version leaves
 * the installation where it is ("skip versions above the ceiling, so an installation inside the
 * range stays inside it").
 */
export const pickAllowedVersion = (policy: VersionPolicy, versions: KnownVersions): UpdatePick =>
  call('pickAllowedVersion', policy, versions);

/** The commands the required range does and does not stop: everything else is a start. */
export const CLAUDE_COMMANDS: readonly ['start', 'update', 'install', 'doctor'] =
  call('CLAUDE_COMMANDS');
export type ClaudeCommand = (typeof CLAUDE_COMMANDS)[number];

export type StartVerdict =
  | { readonly allowed: true }
  | {
      readonly allowed: false;
      readonly reason: 'below-required-minimum' | 'above-required-maximum';
      readonly message: string;
    };

export interface StartRequest {
  readonly command: ClaudeCommand;
  /** The running version. */
  readonly current: string;
}

/**
 * "When the running version is older [newer], Claude Code exits at startup". The check is made once,
 * when Claude Code starts: a session already running continues, so a host calls this at launch only.
 */
export const startGate = (policy: VersionPolicy, request: StartRequest): StartVerdict =>
  call('startGate', policy, request);

/** The `/config` choice when moving from `latest` to `stable`: allow the downgrade, or stay on the current version. */
export type DowngradeAnswer = 'downgrade' | 'stay';

export interface ChannelSwitch {
  /** The channel followed now; picking it again changes nothing. */
  readonly from: Channel;
  readonly to: Channel;
  readonly currentVersion: string;
  /** Needed to leave `latest` for `stable`; without it the switch asks. */
  readonly answer?: DowngradeAnswer;
}

export type SwitchPlan =
  | { readonly ask: 'downgrade-or-stay' }
  | {
      readonly set: { readonly autoUpdatesChannel?: 'stable'; readonly minimumVersion?: string };
      readonly remove: readonly ('autoUpdatesChannel' | 'minimumVersion')[];
    };

/**
 * What the user settings change when the channel is picked under **Auto-update channel** in
 * `/config`: "stable" is written (asking first about the downgrade, and staying sets
 * `minimumVersion`); switching back to latest removes the key and clears `minimumVersion`. Only
 * "from `latest` to `stable`" asks.
 */
export const channelSwitch = (request: ChannelSwitch): SwitchPlan => call('channelSwitch', request);
