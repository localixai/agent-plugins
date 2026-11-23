import { GUARD_CONFIG_ID, GUARD_ID } from '../../protocol/mods-settings.ts';
import type { RegisterTier } from '../types.ts';
import {
  flagSetting,
  isManagedContext,
  listSetting,
  type ModsPolicyInput,
} from './policy-input.ts';
import { whereModsRun } from './where-mods-run.ts';

/**
 * Whether a hooks module may load, and in which tier (plugins/mods/admin.md, "Choose how much to
 * allow"; plugins/mods/troubleshoot.md, "Refusal messages"). A plugin that is not loaded keeps
 * the rest of itself: this decides the hooks module only.
 */

/** What the host knows about a plugin that has a hooks module. */
export interface ModCandidate {
  /** `name@marketplace`, `name@inline` for `--plugin-dir`, or `sec-default@builtin`. */
  readonly id: string;
  /** The manifest name, which names the mod in the chain. */
  readonly name: string;
  /** Built into Claude Code, not installed. */
  readonly isBuiltin: boolean;
  /** What managed settings and the marketplace say about an installed plugin. */
  readonly managed?: {
    /** Managed `enabledPlugins` sets the plugin to `true`. */
    readonly isEnabled: boolean;
    /** The absolute directory of the marketplace, when managed settings name it as a directory. */
    readonly marketplaceDirectory?: string;
    /** The marketplace lists the plugin by a relative path, so Claude Code loads it in place. */
    readonly isLoadedInPlace: boolean;
    /**
     * Managed `extraKnownMarketplaces` declares the plugin's marketplace: with the plugin's id in a
     * managed `prependPlugins` or `appendPlugins`, a mod from a remote marketplace counts as the
     * organization's (§14.9.5, Claude Code 2.1.324 or later).
     */
    readonly isMarketplaceManaged?: boolean;
  };
  /** The plugin reaches a cloud session (overview.md, "Where mods run"): only then do its hooks run there. */
  readonly reachesCloudSession?: boolean;
  /** Names of the plugins this one lists under `dependencies`: it runs before them. */
  readonly dependencies: readonly string[];
}

export interface ModLoadDecision {
  readonly load: boolean;
  readonly tier: RegisterTier;
  /** Why the module is not loaded, as the debug log gives it after `hooks module <id> not loaded:`. */
  readonly reason?: string;
}

const isAbsolute = (path: string, platform: 'posix' | 'windows'): boolean =>
  platform === 'windows' ? /^(?:[A-Za-z]:[\\/]|\\\\)/.test(path) : path.startsWith('/');

/**
 * A mod from a directory on the machine counts as the organization's when managed `enabledPlugins`
 * enables it, managed settings name its marketplace as a directory by absolute path, and the
 * marketplace lists it by a relative path, so it loads in place (§14.9.5, the second case).
 */
function isOrganizationsDirectory(candidate: ModCandidate, platform: 'posix' | 'windows'): boolean {
  const managed = candidate.managed;
  const directory = managed?.marketplaceDirectory;
  return (
    managed?.isEnabled === true &&
    managed.isLoadedInPlace &&
    directory !== undefined &&
    isAbsolute(directory, platform)
  );
}

/** Whether a managed `prependPlugins` or `appendPlugins` lists `id`. */
const managedListNames = (policy: ModsPolicyInput, id: string): boolean =>
  (['prependPlugins', 'appendPlugins'] as const).some((name) => {
    const list = listSetting(policy, name);
    return list?.isManaged === true && list.ids.includes(id);
  });

/**
 * A mod from a remote marketplace (a GitHub repository, for one) counts as the organization's when
 * managed settings enable its plugin, declare the marketplace in `extraKnownMarketplaces` and list the
 * mod's id in `prependPlugins` or `appendPlugins` (§14.9.5, the first case; Claude Code 2.1.324).
 */
function isOrganizationsRemote(candidate: ModCandidate, policy: ModsPolicyInput): boolean {
  const managed = candidate.managed;
  return (
    managed?.isEnabled === true &&
    managed.isMarketplaceManaged === true &&
    managedListNames(policy, candidate.id)
  );
}

/**
 * Whether a mod counts as the organization's, in the two cases of §14.9.5 and no other; a plugin
 * managed settings enable that meets neither counts as a user's.
 */
export function countsAsOrganizations(candidate: ModCandidate, policy: ModsPolicyInput): boolean {
  if (candidate.isBuiltin) return false;
  return (
    isOrganizationsDirectory(candidate, policy.platform) || isOrganizationsRemote(candidate, policy)
  );
}

/** Why a plugin managed settings enable counts as a user's: the words after `but` are ours (judgment). */
function whyUsers(managed: NonNullable<ModCandidate['managed']>): string {
  if (managed.isMarketplaceManaged === true) {
    return 'no managed prependPlugins or appendPlugins lists it';
  }
  return managed.isLoadedInPlace
    ? 'managed settings name no marketplace directory for it by absolute path'
    : 'it is copied into the plugin cache and managed settings do not declare its marketplace';
}

/**
 * The debug line of a plugin managed settings enable whose mod still counts as a user's: it starts
 * with the plugin's id and `is enabled by managed settings, but`, saying why (§14.9.5 item 1).
 * `undefined` for any other plugin.
 */
export function managedButUsers(
  candidate: ModCandidate,
  policy: ModsPolicyInput,
): string | undefined {
  const { managed } = candidate;
  if (candidate.isBuiltin || managed?.isEnabled !== true) return undefined;
  if (countsAsOrganizations(candidate, policy)) return undefined;
  return `${candidate.id} is enabled by managed settings, but ${whyUsers(managed)}, so its mod counts as a user's`;
}

/** The guard's id in either form `prependPlugins` accepts. */
export const isGuardId = (id: string): boolean => id === GUARD_ID || id === GUARD_CONFIG_ID;

/**
 * Where the guard loads: on a machine with managed settings, or in a session whose credential is a
 * Team or Enterprise `/login`. If managed `prependPlugins` is set, the guard loads only when that
 * list names it.
 */
export function guardLoads(policy: ModsPolicyInput): boolean {
  if (!isManagedContext(policy)) return false;
  const list = policy.managed?.prependPlugins;
  return list === undefined || list.some(isGuardId);
}

const ROLLOUT_WORDS = {
  'served-off': 'the rollout switch served off',
  'saved-off': 'the rollout switch was saved off by an earlier session',
} as const;

/** The reason the rollout switch is off, as the debug log gives it for an installed plugin (troubleshoot.md, "Refusal messages"). */
const rolloutRefusal = (rollout: NonNullable<ModsPolicyInput['rollout']>): string =>
  `hooks modules are turned off for installed plugins in this process: ${ROLLOUT_WORDS[rollout]}`;

/** What keeps every mod out until the trust prompt is answered: the wording is ours (judgment). */
const UNTRUSTED =
  'the directory is not trusted yet: no mod loads until the trust prompt is answered';

/** Why the place the session runs in keeps the plugin's hooks from running (overview.md, "Where mods run"), if it does. */
function placeRefusal(policy: ModsPolicyInput, candidate: ModCandidate): string | undefined {
  if (policy.where === undefined) return undefined;
  const place = whereModsRun({
    ...policy.where,
    reachesCloud: candidate.reachesCloudSession === true,
  });
  return place.hooksRun ? undefined : place.hooksReason;
}

/** The refusal text of an installed plugin's hooks module that policy keeps out, or `undefined`. */
function installedRefusal(
  policy: ModsPolicyInput,
  candidate: ModCandidate,
  isOrganizations: boolean,
): string | undefined {
  const placed = placeRefusal(policy, candidate);
  if (placed !== undefined) return placed;
  if (policy.isTrusted === false) return UNTRUSTED;
  if (policy.flags.safeMode === true) {
    return 'installed plugins load no hooks module in this mode (--safe-mode)';
  }
  if (policy.rollout !== undefined) return rolloutRefusal(policy.rollout);
  // The key reaches every installed plugin, the organization's too, in managed settings.
  if (flagSetting(policy.managed, 'disableAllHooks')) return 'disableAllHooks in managed settings';
  if (isOrganizations) return undefined;
  const narrowed =
    flagSetting(policy.managed, 'allowManagedHooksOnly') ||
    flagSetting(policy.user, 'disableAllHooks');
  if (narrowed) return 'only managed plugins and built-in plugins run';
  if (policy.flags.bare === true) {
    return 'installed plugins that are not managed load no hooks module in this mode (--bare)';
  }
  return undefined;
}

/** Where an installed plugin's hooks module runs: the lists in settings, else the organization's before users'. */
function installedTier(policy: ModsPolicyInput, candidate: ModCandidate): RegisterTier {
  const isOrganizations = countsAsOrganizations(candidate, policy);
  const append = listSetting(policy, 'appendPlugins');
  const prepend = listSetting(policy, 'prependPlugins');
  // A list in managed settings names organization mods only: "Claude Code skips an id whose plugin
  // doesn't meet the conditions"; a list in the user's settings orders the user's own mods.
  const names = (list: typeof prepend): boolean =>
    list !== undefined && list.isManaged === isOrganizations && list.ids.includes(candidate.id);
  // "An id listed in both keys is prepended" (the `prependPlugins` description in the SDK's `Settings`).
  if (names(prepend)) return 'prepend';
  if (names(append)) return 'append';
  return isOrganizations ? 'prepend' : 'user';
}

/** The tier a built-in mod runs in: the guard first among `prepend`, other built-in mods last. */
const builtinTier = (candidate: ModCandidate): RegisterTier =>
  isGuardId(candidate.id) ? 'prepend' : 'builtin';

/**
 * Whether `candidate`'s hooks module loads under `policy`, and in which tier. The refusals are the
 * documented lines; the guard's own refusal of a user's mod (`allowManagedModsOnly`) is not here: it
 * is the guard's answer to `plugin.register`, after the module loaded.
 */
export function decideModLoad(policy: ModsPolicyInput, candidate: ModCandidate): ModLoadDecision {
  if (candidate.isBuiltin) {
    const loads = !isGuardId(candidate.id) || guardLoads(policy);
    return {
      load: loads,
      tier: builtinTier(candidate),
      ...(loads
        ? {}
        : { reason: 'the built-in guard loads only where an organization manages Claude Code' }),
    };
  }
  const reason = installedRefusal(policy, candidate, countsAsOrganizations(candidate, policy));
  const tier = installedTier(policy, candidate);
  return reason === undefined ? { load: true, tier } : { load: false, tier, reason };
}

/** What `claude plugin test` and the troubleshooting table call the reason mods are turned off in this process (troubleshoot.md, "Check whether mods can load"). */
const ROLLOUT_REASONS = {
  'served-off': 'hooks modules are turned off in this process: the rollout switch served off',
  'saved-off':
    'hooks modules are turned off in this process: the rollout switch was saved off by an earlier session',
} as const;

const TURNED_OFF_HERE =
  'hooks modules are turned off here (disableAllHooks, allowManagedHooksOnly or a policy)';

/**
 * Why a shell command that loads the tester's own mods cannot: `disableAllHooks` in the user's or
 * the organization's settings, or managed `allowManagedHooksOnly` ("here"), or the rollout switch
 * ("in this process"). `undefined` when mods can load. `allowManagedModsOnly` is not reported ("which
 * this command doesn't report"). The line is Claude Code 2.1.325's, which names no one setting
 * (Agent Plugins Extended §10.42.1).
 */
export function hooksModulesOff(policy: ModsPolicyInput): string | undefined {
  if (policy.rollout !== undefined) return ROLLOUT_REASONS[policy.rollout];
  const isOff =
    flagSetting(policy.managed, 'disableAllHooks') ||
    flagSetting(policy.managed, 'allowManagedHooksOnly') ||
    flagSetting(policy.user, 'disableAllHooks');
  return isOff ? TURNED_OFF_HERE : undefined;
}
