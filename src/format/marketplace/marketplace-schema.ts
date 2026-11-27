import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { MarketplaceMetadata, Owner } from './types.ts';
import type { Renames } from './renames.ts';

/**
 * The top level of `marketplace.json` (plugins/marketplace-reference.md, "Top-level fields" and
 * "Validation messages"): every key Claude Code reads, the type of each, and the report for a key
 * that is missing, of the wrong type or unknown. `name` and `owner.name` are checked for their
 * grammar by the caller; `plugins` is handed on entry by entry.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/marketplace_schema.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/marketplace/marketplace-schema');

/** `Invalid input: expected <type>, received <type>`, the one wording of a value of the wrong type. */
export const mismatch = (path: string, expected: string, value: unknown): PluginIssue =>
  call('mismatch', path, expected, value);

/** The top-level fields of a marketplace, everything but its entries. */
export interface TopLevel {
  readonly name: string;
  readonly owner: Owner;
  readonly plugins: readonly unknown[];
  readonly description?: string;
  readonly version?: string;
  readonly metadata?: MarketplaceMetadata;
  readonly forceRemoveDeletedPlugins: boolean;
  readonly allowCrossMarketplaceDependenciesOn: readonly string[];
  readonly renames: Renames;
}

export interface TopLevelResult {
  /** Undefined when any field breaks the schema: such a marketplace does not load. */
  readonly top: TopLevel | undefined;
  readonly issues: readonly PluginIssue[];
}

/** Reads and checks every top-level key of the parsed `marketplace.json`. */
export const readTopLevel = (raw: Readonly<Record<string, unknown>>): TopLevelResult =>
  call('readTopLevel', raw);
