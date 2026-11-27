import type { Clock } from '../ports/clock.ts';
import type { PluginStateStore, StateStoreFailure } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The dated copies beside `installed_plugins.json` age out (plugins/loading.md, "Find plugins on
 * disk"; plugins/troubleshooting.md: "the copies age out on the `cleanupPeriodDays` schedule"). The
 * setting: "Claude Code deletes the files in the paths below once they're older than
 * `cleanupPeriodDays` ... The default is 30 days and the minimum is 1; setting `0` fails with a
 * validation error" (claude-directory.md, "Cleaned up automatically").
 *
 * Judgments (spec/judgments.md): a copy is removed once it is strictly older than the period, by
 * the time it was last written; a period that is not a whole number of at least 1 is the default,
 * since the setting would not have validated; only the two kinds of dated copy are ever removed.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/copy_sweep.rs`,
 * docs/RUST.md).
 */

const call = coreModule('marketplace/copy-sweep');

export const DEFAULT_CLEANUP_PERIOD_DAYS: number = call('DEFAULT_CLEANUP_PERIOD_DAYS');

/** A name directly under the plugins root and when it was last written, in Unix milliseconds. */
export interface AgedName {
  readonly name: string;
  readonly modifiedAtMs: number;
}

/** The dated copies among `entries` that are older than `cleanupPeriodDays` at `nowMs`. */
export const agedCopies = (input: {
  readonly entries: readonly AgedName[];
  readonly nowMs: number;
  readonly cleanupPeriodDays?: number | undefined;
}): readonly string[] => call('agedCopies', input);

export interface CopySweep {
  readonly removed: readonly string[];
  /** The removals that failed; the copies stay and are tried again on the next sweep. */
  readonly failures: readonly StateStoreFailure[];
}

/** Lists the root, looks up the age of each dated copy, and removes those that aged out. */
export const sweepAgedCopies = (
  ports: { readonly store: PluginStateStore; readonly clock: Clock },
  options: { readonly cleanupPeriodDays?: number | undefined },
  { signal }: { readonly signal: AbortSignal },
): Promise<CopySweep> => call.async('sweepAgedCopies', ports, options, { signal });
