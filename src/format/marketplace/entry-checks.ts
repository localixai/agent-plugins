import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { MarketplaceEntry, PluginSource, ReadMode } from './types.ts';

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

/**
 * The reports on a name, in two parts: what is wrong with its shape, which an entry cannot be
 * read past, and the reserved-name ruling, which leaves the entry otherwise sound.
 */
export const nameReports = (
  name: string,
  path: string,
  mode: ReadMode,
  marketplace?: string,
): { readonly shape: PluginIssue[]; readonly reserved: PluginIssue[] } =>
  call('nameReports', name, path, mode, marketplace);

export interface HeaderFields {
  readonly name: string;
  readonly strict: boolean;
  readonly headers?: unknown;
  readonly headersHelper?: unknown;
}

/** The header reports of an entry: the fields on a source that ignores them, or the ones that apply to an archive. */
export const headerIssues = (
  entry: HeaderFields,
  source: PluginSource,
  base: string,
): PluginIssue[] => call('headerIssues', entry, source, base);

/**
 * "Entry declares version "x" but <path>/plugin.json says "y". At install time, plugin.json wins",
 * for a relative-path entry, "whose plugin files are inside the marketplace itself", when the two
 * versions differ.
 */
export const versionIssues = (
  version: string | undefined,
  source: PluginSource,
  manifest: unknown,
  base: string,
): PluginIssue[] => call('versionIssues', version, source, manifest, base);
