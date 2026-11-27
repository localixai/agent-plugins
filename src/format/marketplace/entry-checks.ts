import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { MarketplaceEntry } from './types.ts';

/**
 * The checks of a plugin entry beyond its field types (plugins/marketplace-reference.md,
 * "Validation messages"): the name, a name used twice, the archive-only header fields, and a
 * version that plugin.json overrides.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/entry_checks.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/marketplace/entry-checks');

/** A kept entry and the place it had in `plugins`. */
export interface IndexedEntry {
  readonly index: number;
  readonly entry: MarketplaceEntry;
}

/**
 * Two entries with one name: "Duplicate plugin name "x" found in marketplace". The first keeps
 * the name; a later one is dropped alone (the docs say each entry is validated on its own and
 * leave the pair open: spec/judgments.md).
 */
export const withoutDuplicates = (
  read: readonly IndexedEntry[],
): {
  entries: MarketplaceEntry[];
  issues: PluginIssue[];
} => call('withoutDuplicates', read);
