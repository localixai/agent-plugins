import type { Clock } from '../ports/clock.ts';
import type { Environment } from '../ports/environment.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { AutoUpdateCandidate, SkippedBy } from './auto-update-skips.ts';

/**
 * The auto-update pass (plugins/loading.md, "When auto-update runs"): "In an interactive session,
 * after you send your first message, Claude Code waits a random delay of up to ten minutes. It then
 * refreshes every marketplace with auto-update on and updates the plugins installed from them on
 * disk." The kit plans one pass and arms the first one on the `Clock`; how often a host repeats it
 * is the host's choice (the docs describe the one pass of a session).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/updates/auto_update_pass.rs`, docs/RUST.md).
 */

const call = coreModule('updates/auto-update-pass');

export const FIRST_PASS_MAX_DELAY_MS: number = call('FIRST_PASS_MAX_DELAY_MS');

/** A delay in `[0, 10 minutes]` from a source of numbers in `[0, 1]` (anything outside is clamped). */
export const firstPassDelayMs = (random: () => number): number => call('firstPassDelayMs', random);

export interface FirstPassOptions {
  readonly clock: Clock;
  readonly env: Environment;
  /** The host's source of randomness: the kit has none (L3). */
  readonly random: () => number;
  readonly run: () => void;
}

export interface ArmedPass {
  /** False when the environment turns the pass off: nothing was set on the clock. */
  readonly armed: boolean;
  readonly cancel: () => void;
}

/** Call this after the first message. The pass is armed only when `autoUpdatePassAllowed`. */
export const armFirstPass = (options: FirstPassOptions): ArmedPass => call('armFirstPass', options);

export interface PassMarketplace {
  readonly name: string;
  /** Its resolved auto-update setting (`marketplaceAutoUpdate`). */
  readonly autoUpdate: boolean;
  /** The plugins the user installed from it. */
  readonly plugins: readonly { readonly id: string; readonly candidate: AutoUpdateCandidate }[];
}

export interface PassPlan {
  /** Marketplaces to refresh, in the order given. */
  readonly refresh: readonly string[];
  /** Plugin ids to update on disk. */
  readonly update: readonly string[];
  readonly skipped: readonly {
    readonly id: string;
    readonly reason: SkippedBy;
    readonly listedInErrorsTab: boolean;
  }[];
}

/** One pass: nothing when the environment turns it off, otherwise per auto-update marketplace its plugins less the skipped. */
export const planAutoUpdatePass = (
  marketplaces: readonly PassMarketplace[],
  env: Environment,
): PassPlan => call('planAutoUpdatePass', marketplaces, env);

/** What the running session shows after a plugin updated on disk: it keeps the version it loaded. */
export const pluginUpdatedNotice = (name: string): string => call('pluginUpdatedNotice', name);
