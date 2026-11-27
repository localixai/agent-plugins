import { coreModule } from '../wasm/bridge.ts';

/**
 * Which plugin loads when enabled plugins from different origins share a manifest name
 * (plugins/loading.md, "Name conflicts", "Hooks when two enabled plugins share a name", "Keep a
 * session-only plugin from loading"). Pure over what the host found. The rows and messages the docs
 * quote are the docs'; the wording of the rest is the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/name_conflicts.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/name-conflicts');

export type PluginOrigin = 'session-only' | 'installed' | 'skills-dir' | 'synced';

export interface NameCandidate {
  /** `name@marketplace`, `name@inline`, `name@skills-dir`. */
  readonly id: string;
  /** The manifest name every copy of the conflict shares. */
  readonly name: string;
  readonly origin: PluginOrigin;
  /** Enabled in the settings that apply; `"<id>": false` disables a session-only plugin too. */
  readonly enabled: boolean;
  /** The id appears in managed settings `enabledPlugins`, as `true` or `false`. */
  readonly managedLocked?: boolean;
  /** A skills-directory plugin's scope and path, to say which copy shadowed which. */
  readonly skillsDir?: { readonly scope: 'user' | 'project'; readonly path: string };
}

export interface NameVerdict {
  readonly id: string;
  readonly loads: boolean;
  /** The row of the `/plugin` **Errors** tab, or the line printed, for a copy that did not load. */
  readonly note?: string;
  /** Recorded only by the `--debug` log: the installed copy a session-only plugin replaced silently. */
  readonly debugLog?: string;
}

/**
 * The copy of a name that loads and what each other copy reports. A disabled copy never holds the name
 * ("A disabled session-only plugin doesn't shadow"), except that a managed-locked id keeps a
 * `--plugin-dir` copy of its name out even when its value is `false`.
 */
export const resolveNameConflict = (candidates: readonly NameCandidate[]): readonly NameVerdict[] =>
  call('resolveNameConflict', candidates);

/** One copy of a plugin name that declares hooks. */
export interface HookCopy {
  readonly id: string;
  /** Managed settings turn this copy on. */
  readonly managedEnabled: boolean;
  /** The order Claude Code loads the copies in. */
  readonly loadOrder: number;
  readonly hooksJson: boolean;
  readonly hooksModule: boolean;
}

export interface HookNote {
  readonly id: string;
  readonly kind: 'hooks.json' | 'hooks module';
  /** The note of the **Errors** tab and of the copy's details. */
  readonly message: string;
}

/** "When your organization's managed settings turn on one of the copies, that copy holds the name. Otherwise the copy Claude Code loads first holds it." */
export const nameHolder = (copies: readonly HookCopy[]): HookCopy | undefined =>
  call('nameHolder', copies);

/** The notes for the copies whose hooks are left out; one plugin per name registers `hooks/hooks.json` hooks and one loads a hooks module. */
export const hookConflictNotes = (copies: readonly HookCopy[]): readonly HookNote[] =>
  call('hookConflictNotes', copies);

/** `--plugin-dir` plugin ids are `<name>@inline`: the entry that keeps one from loading. */
export const sessionOnlyOffEntry = (name: string): Readonly<Record<string, false>> =>
  call('sessionOnlyOffEntry', name);

/** Disabling the holder (a copy managed settings do not turn on) and reloading registers the remaining copy's hooks. */
export const afterDisabling = (
  copies: readonly HookCopy[],
  disabledId: string,
): { readonly allowed: boolean; readonly notes: readonly HookNote[] } =>
  call('afterDisabling', copies, disabledId);
