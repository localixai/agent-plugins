import type { InstallRecord } from './installed-plugins.ts';
import type { PathJoin, StoreLayout } from './store-layout.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Cleanup of previous versions (plugins/loading.md): "When you update or uninstall a plugin, Claude
 * Code writes an `.orphaned_at` marker into the previous version directory. It removes that
 * directory in a background cleanup 14 days later, so a session that already loaded the old version
 * keeps running. The sweep runs only while `installed_plugins.json` records at least one install."
 * Pure: which directories to mark and which to remove; the host writes the marker, reads its time
 * and deletes. Judgments (spec/judgments.md): a directory is swept at exactly 14 days; a directory
 * a remaining record points at is neither marked nor swept, since it is in use again; only a
 * version directory of the cache, exactly `cache/<marketplace>/<plugin>/<version>`, is marked or
 * swept: a plugin loaded in place points at its own source directory, and a sweep of it, of the
 * cache or of a marketplace folder would delete what is not an old version.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/orphan_cleanup.rs`,
 * docs/RUST.md).
 */

const call = coreModule('marketplace/orphan-cleanup');

export const ORPHANED_MARKER: string = call('ORPHANED_MARKER');

/** 14 days. */
export const ORPHAN_GRACE_MS: number = call('ORPHAN_GRACE_MS');

export const orphanMarkerPath = (versionDir: string, join: PathJoin): string =>
  call('orphanMarkerPath', versionDir, join);

/**
 * The version directories an update or uninstall leaves behind: those of `removed` inside the cache
 * that no `remaining` record uses.
 */
export const dirsToOrphan = (
  removed: readonly InstallRecord[],
  remaining: readonly InstallRecord[],
  layout: StoreLayout,
): readonly string[] => call('dirsToOrphan', removed, remaining, layout);

export interface Orphan {
  readonly dir: string;
  /** When the marker was written, in Unix milliseconds. */
  readonly orphanedAtMs: number;
}

/** The directories the background cleanup removes now. */
export const orphanSweep = (input: {
  /** Every install `installed_plugins.json` still records. */
  readonly remaining: readonly InstallRecord[];
  readonly orphans: readonly Orphan[];
  readonly nowMs: number;
  readonly layout: StoreLayout;
}): readonly string[] => call('orphanSweep', input);
