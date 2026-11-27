import type { Applies } from '../general/edit-effect.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { Mechanism } from './delivery.ts';
import type { HelperPlan } from './compose.ts';

/**
 * Where and when a deployed policy applies (managed-settings.md, "Where and when a policy applies"):
 * which surfaces and places read which sources, what reaches a running session, and what waits for
 * the developer. Pure decisions over facts the host copies in.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/applicability.rs`,
 * docs/RUST.md).
 */

const call = coreModule('settings/managed/applicability');

export const POLICY_SURFACES: readonly [
  'terminal',
  'vscode',
  'jetbrains',
  'desktopCodeTab',
  'agentSdk',
] = call('POLICY_SURFACES');
export type PolicySurface = (typeof POLICY_SURFACES)[number];

/** The settings files an Agent SDK app chooses among with `settingSources`. */
export const SDK_SETTING_SOURCES: readonly ['user', 'project', 'local'] =
  call('SDK_SETTING_SOURCES');
export type SdkSettingSource = (typeof SDK_SETTING_SOURCES)[number];

export interface LoadedSources {
  readonly managed: boolean;
  readonly files: readonly SdkSettingSource[];
}

/**
 * What a surface loads. "On the developer's machine, the terminal, the VS Code and JetBrains
 * extensions, the desktop app's Code tab, and Agent SDK sessions read all of these sources. Agent
 * SDK sessions load managed settings even when `settingSources` excludes the user, project, and
 * local files."
 */
export const sourcesLoadedBy = (
  surface: PolicySurface,
  settingSources?: readonly SdkSettingSource[],
): LoadedSources => call('sourcesLoadedBy', surface, settingSources);

export const PLACES: readonly [
  'machine',
  'cloudHosted',
  'cloudSelfHosted',
  'tagHosted',
  'tagSelfHosted',
  'coworkOnMachine',
  'coworkFullVm',
  'coworkRemote',
] = call('PLACES');
export type Place = (typeof PLACES)[number];

/** How much of the managed settings file a place applies. */
export const FILE_USES: readonly ['all', 'crossSourceKeys', 'none'] = call('FILE_USES');
export type FileUse = (typeof FILE_USES)[number];

export interface PlaceFacts {
  /** Server-managed settings deliver at least one policy key to this session. */
  readonly serverDeliversPolicyKey: boolean;
  /** `managedSourcesBehavior` is `"merge"`. */
  readonly merging: boolean;
}

export interface PlaceReach {
  /** The session fetches server-managed settings. */
  readonly remote: boolean;
  /** The session reads the device's MDM profile. */
  readonly mdm: boolean;
  readonly file: FileUse;
}

/**
 * The sources each place reads. Cloud sessions in an Anthropic-hosted environment read neither
 * device policy; Claude Tag sessions receive no server-managed settings; Cowork never fetches
 * them, and reads the device's policy only when it runs on the user's machine.
 */
export const reachOfPlace = (place: Place, facts: PlaceFacts): PlaceReach =>
  call('reachOfPlace', place, {
    serverDeliversPolicyKey: facts.serverDeliversPolicyKey,
    merging: facts.merging,
  });

/**
 * Wherever a Cowork session runs, "claude.ai applies the admin console's `strictKnownMarketplaces`
 * and `blockedMarketplaces` lists itself" when someone adds a marketplace from a git repository on
 * claude.ai or from Customize.
 */
export const claudeAiAppliesMarketplaceLists = (place: Place): boolean =>
  call('claudeAiAppliesMarketplaceLists', place);

export interface FetchFacts {
  /** The session authenticates to Anthropic's API directly with an eligible credential. */
  readonly eligibleCredential: boolean;
  /** The provider is Anthropic's API, not Bedrock, Vertex, Foundry or another. */
  readonly anthropicProvider: boolean;
  /** `ANTHROPIC_BASE_URL` points somewhere other than Anthropic's API. */
  readonly baseUrlElsewhere: boolean;
  /** The developer signed in to a gateway with `/login`. */
  readonly gatewayLogin: boolean;
  /** Claude Desktop's Code tab runs behind a gateway that serves that session settings. */
  readonly desktopGatewaySession: boolean;
}

/**
 * Whether remote settings, the top-ranked managed source, are fetched at all (managed-settings.md,
 * "How Claude Code combines managed sources", rank 1): "only when the session authenticates to
 * Anthropic's API directly with an eligible credential, signs in to a gateway with `/login`, or runs
 * in Claude Desktop's Code tab behind a gateway that serves that session settings. On other
 * providers, or when `ANTHROPIC_BASE_URL` points somewhere other than Anthropic's API, it starts at
 * the next source."
 */
export const fetchesRemoteSettings = (facts: FetchFacts): boolean =>
  call('fetchesRemoteSettings', facts);

/**
 * When a change to `key` reaches a running session: "most changes reach a running session on the
 * schedule in the delivery mechanism table, without a restart"; `forceRemoteSettingsRefresh`,
 * `requiredMinimumVersion` and the keys read once "take effect at the next session start".
 */
export const reachesRunningSession = (mechanism: Mechanism, key?: string): Applies =>
  call('reachesRunningSession', mechanism, key);

/**
 * Whether the policy helper runs now: a new or changed one "takes effect at the next launch", and one
 * server-managed settings shadow at launch "runs as soon as a fetch reports those settings removed".
 */
export const helperRunsNow = (
  plan: HelperPlan,
  fetch: { readonly remoteStillDelivers: boolean },
): boolean =>
  call(
    'helperRunsNow',
    {
      helper: plan.helper,
      runsAtLaunch: plan.runsAtLaunch,
      shadowedByRemote: plan.shadowedByRemote,
    },
    { remoteStillDelivers: fetch.remoteStillDelivers },
  );

export interface ServerChange {
  /** The change sets something "that needs approval", such as a hook or an `env` variable. */
  readonly needsApproval: boolean;
  /** One of "the updates that wait for the next launch". */
  readonly waitsForNextLaunch: boolean;
  /** An IDE extension or the Agent SDK hosts the session, so no dialog can be shown to the developer. */
  readonly hosted: boolean;
}

export const SERVER_CHANGE_TIMINGS: readonly [
  'next-launch',
  'awaits-approval',
  'applies-for-current-run',
  'next-poll',
] = call('SERVER_CHANGE_TIMINGS');
export type ServerChangeTiming = (typeof SERVER_CHANGE_TIMINGS)[number];

/**
 * When a server-managed change applies. "Apart from the updates that wait for the next launch, a
 * server-managed change to a setting that needs approval waits for the developer to accept the
 * dialog in an interactive session, and applies for the current run in a session an IDE extension or
 * the Agent SDK hosts. Other server-managed changes apply on the next poll."
 */
export const serverChangeTiming = (change: ServerChange): ServerChangeTiming =>
  call('serverChangeTiming', change);

export interface MinimumVersionFacts {
  /** The binary is older than `requiredMinimumVersion`. */
  readonly outdated: boolean;
  /** The session is already running. */
  readonly running: boolean;
}

/** "`requiredMinimumVersion` blocks an outdated binary from starting and doesn't end a session that's already running." */
export const minimumVersionEffect = (
  facts: MinimumVersionFacts,
): { blocksStart: boolean; endsSession: boolean } => call('minimumVersionEffect', facts);

export interface StartupFetchFacts {
  readonly forceRemoteSettingsRefresh: boolean;
  /** The session fetches server-managed settings at all. */
  readonly fetchesRemote: boolean;
  /** The developer signs in at startup (first run, after `/logout`). */
  readonly signsInAtStartup: boolean;
  /** Anthropic requires the organization's settings before a session starts. */
  readonly anthropicRequires: boolean;
  /** A Cloud gateway session. */
  readonly gatewaySession: boolean;
  /** A `claude auth` subcommand. */
  readonly authSubcommand: boolean;
}

export interface StartupFetch {
  readonly blocksStartup: boolean;
  /** The session exits when the fetch fails. */
  readonly exitsOnFailure: boolean;
  /** Seconds startup waits for the fetch before it opens the session. */
  readonly waitsUpToSeconds?: number;
}

/**
 * What startup does about the fetch (settings-reference.md, `forceRemoteSettingsRefresh`):
 * `true` blocks until a fresh fetch and exits on failure; unset does not block, though a sign-in
 * startup waits up to five seconds; "Anthropic can require the settings", which blocks a start
 * without them; a Cloud gateway session waits and exits when the gateway cannot be reached. The
 * `claude auth` subcommands are exempt, and a session that does not fetch starts without waiting.
 */
export const startupFetchOf = (facts: StartupFetchFacts): StartupFetch =>
  call('startupFetchOf', facts);
