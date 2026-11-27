import type { ErrorsTabEntry } from './install-messages.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Two questions about `enabledPlugins` that the loading reference answers (plugins/loading.md,
 * "Find where a plugin is enabled"): why a plugin you turned off still loads, and whether Claude
 * Code fetches a plugin only a project enables. Pure over the facts a host read from the settings
 * layers; which layer wins is the settings module's (`enabledPlugin`), so a caller passes its
 * `decidedBy` here, and this module imports nothing from it (same layer).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/enablement_notes.rs`,
 * docs/RUST.md).
 */

const call = coreModule('marketplace/enablement-notes');

/** The sources an `enabledPlugins` entry can come from (the table of "Find where a plugin is enabled"). */
export type EnableSource = 'addDir' | 'user' | 'project' | 'local' | 'flag' | 'managed';

// ---- "Disabled in user settings but still loads" ----

export interface UserDisableOverridden {
  /** The row's text in `claude plugin list` and `/plugin`. */
  readonly message: string;
  /** How to turn it off on this machine: `false` for the id in this file, which outranks the project file. */
  readonly optOut?: { readonly file: string; readonly value: false };
}

/**
 * `Disabled in ~/.claude/settings.json but still loads — project settings enable it, which
 * overrides your user setting`, for a plugin set to `false` in user settings that a `true` in a
 * higher source keeps on (loading.md; troubleshooting.md). Judgment (spec/judgments.md): the docs
 * quote the sentence for `project` only, so another source is named in the same place by its
 * label (`cli flag settings enable it`).
 */
export const overriddenUserDisable = (winner: {
  readonly user?: boolean;
  readonly decidedBy: EnableSource;
  readonly on: boolean;
}): UserDisableOverridden | undefined => call('overriddenUserDisable', winner);

// ---- "Enabled in project settings but not installed" ----

/** One source that sets the plugin to `true`; a `local` file says whether git tracks it. */
export interface TrueIn {
  readonly source: EnableSource;
  readonly gitTracked?: boolean;
}

export interface EnabledFacts {
  readonly name: string;
  readonly marketplace: string;
  /** Every source that sets the plugin to `true`. */
  readonly trueIn: readonly TrueIn[];
  /** An install record exists for it. */
  readonly installed: boolean;
  /** Its marketplace entry has a relative-path source. */
  readonly relativePath: boolean;
  /** A seed directory already holds it. */
  readonly seeded: boolean;
}

export interface EnabledFetchDecision {
  /** Claude Code downloads the plugin's external source. */
  readonly fetch: boolean;
  /** It loads from the marketplace itself, needing no install record. */
  readonly inPlace: boolean;
  /** The Errors tab entry, when a project enables a plugin that is not here. */
  readonly error?: ErrorsTabEntry;
}

/**
 * What Claude Code does with an enabled plugin that has no install record on this machine. A
 * relative-path plugin "needs no install record because it loads from the marketplace itself", and
 * a plugin a seed directory holds is there; otherwise an external source is fetched only when user
 * settings, an untracked `.claude/settings.local.json`, `--settings` or managed settings set it to
 * `true`. When only the project's `.claude/settings.json` does, nothing is fetched and the Errors
 * tab says so. Judgment (spec/judgments.md): a `true` from `--add-dir` alone neither fetches nor
 * reports, since the docs name it in neither list.
 */
export const decideEnabledFetch = (facts: EnabledFacts): EnabledFetchDecision =>
  call('decideEnabledFetch', facts);
