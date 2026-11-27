/** Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/delivery.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { Settings } from '../../protocol/official.ts';
import type { ManagedSource } from './admin-source.ts';

const call = coreModule('settings/managed/delivery');

/**
 * The delivery mechanisms as data (managed-settings.md, "Deploy a managed settings file", "Choose a
 * delivery mechanism", "Where each mechanism stores the policy"): where each stores the policy, how
 * often Claude Code reads it, and which source of the composition it is.
 */
export const PLATFORMS: readonly ['macos', 'linux', 'wsl', 'windows'] = call('PLATFORMS');

export type Platform = (typeof PLATFORMS)[number];

export const MECHANISMS: readonly ['serverManaged', 'mdm', 'file', 'hkcu'] = call('MECHANISMS');

export type Mechanism = (typeof MECHANISMS)[number];

export interface MechanismTraits {
  /** The sources of the composition this mechanism fills, on a machine that has it. */
  readonly sources: readonly ManagedSource[];
  /** "Read at startup" holds for every mechanism. */
  readonly readAtStartup: boolean;
  /** Minutes between checks for a change; absent when the mechanism is not polled. */
  readonly pollEveryMinutes?: number;
  /** "Reloaded when a file changes". */
  readonly reloadsOnChange: boolean;
  /** "The MDM and file rows are together called endpoint-managed settings, because the policy is stored on the developer's device." */
  readonly endpointManaged: boolean;
}

export const MECHANISM_TRAITS: { readonly [M in Mechanism]: MechanismTraits } =
  call('MECHANISM_TRAITS');

export interface RereadFacts {
  readonly minutesSinceRead: number;
  /** The file changed on disk since it was read. */
  readonly changed: boolean;
}

/** Whether Claude Code reads the mechanism again now: on its poll interval, or when its file changes. */
export const rereadDue = (mechanism: Mechanism, facts: RereadFacts): boolean =>
  call('rereadDue', mechanism, facts);

/** The MDM mechanism a platform has: a configuration profile on macOS, the `HKLM` registry on Windows; Linux has neither. */
export const MDM_SOURCE: { readonly [P in Platform]: ManagedSource | undefined } =
  call('MDM_SOURCE');

/** "The `com.anthropic.claudecode` managed preferences domain." */
export const PLIST_DOMAIN: 'com.anthropic.claudecode' = call('PLIST_DOMAIN');

/** The registry: "the JSON as a `REG_SZ` or `REG_EXPAND_SZ` value named `Settings` under `SOFTWARE\Policies\ClaudeCode`". */
export const REGISTRY_VALUE: {
  readonly name: 'Settings';
  readonly types: readonly ['REG_SZ', 'REG_EXPAND_SZ'];
  readonly key: 'SOFTWARE\\Policies\\ClaudeCode';
} = call('REGISTRY_VALUE');

export const HIVES: readonly ['HKLM', 'HKCU'] = call('HIVES');

export type Hive = (typeof HIVES)[number];

/** The registry path of a hive's policy value, as `/status` and errors write it. */
export const registryPath = (hive: Hive): string => call('registryPath', hive);

export interface ManagedFilePaths {
  readonly dir: string;
  /** `managed-settings.json`. */
  readonly settings: string;
  /** `managed-settings.d/`, the drop-in directory. */
  readonly dropIns: string;
  /** `managed-mcp.json`. */
  readonly mcp: string;
}

/** Where the file-based mechanism keeps its files. */
export const managedFilePaths = (platform: Platform): ManagedFilePaths =>
  call('managedFilePaths', platform);

/** Whether Claude Code reads `path` as the managed settings file of `platform`. */
export const readsManagedSettingsAt = (platform: Platform, path: string): boolean =>
  call('readsManagedSettingsAt', platform, path);

export interface SymlinkFacts {
  /** The deployed file's path. */
  readonly path: string;
  /** Whether the link's target is inside the system directory; `undefined` when Claude Code cannot confirm it. */
  readonly targetInside: boolean | undefined;
}

export interface SymlinkVerdict {
  /** "The policy still applies" whatever the link leads to. */
  readonly applies: boolean;
  /** The warning `/status` and `claude doctor` show, naming the file. */
  readonly warning?: PluginIssue;
}

/**
 * A managed file deployed as a symbolic link: "Claude Code reads the file the link leads to. The
 * policy still applies when the link leads outside the system directory or Claude Code can't confirm
 * that it stays inside. A warning naming the file then appears in `/status` and `claude doctor`."
 */
export const symlinkVerdict = (facts: SymlinkFacts): SymlinkVerdict =>
  call('symlinkVerdict', facts);

/** What one fetch of server-managed settings came to: the policy it delivered, or a failure. */
export type ServerFetch =
  | { readonly ok: true; readonly settings: Settings | undefined }
  | { readonly ok: false };

/**
 * The local cache of server-managed settings: "Claude Code keeps a local cache that it applies at
 * startup and replaces on each successful fetch", so an edit to the cache "lasts only until the next
 * successful fetch" (managed-settings.md, "Where each mechanism stores the policy", "What a developer
 * can change"). A failed fetch leaves the cache as it is.
 */
export const cacheAfterFetch = (
  cached: Settings | undefined,
  fetch: ServerFetch,
): Settings | undefined => call('cacheAfterFetch', cached, fetch);

/** How long a developer's edit of a managed source stays. */
export const EDIT_OUTCOMES: readonly ['stays', 'until-next-fetch', 'refused'] =
  call('EDIT_OUTCOMES');

export type EditOutcome = (typeof EDIT_OUTCOMES)[number];

export const editOfManagedSource = (mechanism: Mechanism, localAdmin: boolean): EditOutcome =>
  call('editOfManagedSource', mechanism, localAdmin);
