/**
 * Reading `marketplace.json` as Claude Code loads it (plugins/marketplace-reference.md, "Top-level
 * fields" and "Validation messages"; plugins/create-marketplace.md, "Problems that validation
 * reports"): the top-level fields and the marketplace's name are checked here; each entry is handed
 * to an `EntryReader` on its own, because "each entry is validated on its own, so one invalid
 * entry doesn't fail the marketplace".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/marketplace.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../issues.ts';
import type {
  Marketplace,
  MarketplaceContext,
  MarketplaceEntry,
  MarketplaceResult,
  ReadMode,
} from './types.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/marketplace/marketplace');

/** What reading one entry gave: the entry when it loads, and what was found either way. */
export interface EntryReading {
  readonly entry: MarketplaceEntry | undefined;
  readonly issues: readonly PluginIssue[];
  /** The entry's fields pass the schema; absent is taken as true. */
  readonly schemaSound?: boolean;
}

/** Where an entry stands in its marketplace. */
export interface EntrySlot {
  readonly index: number;
  /** `metadata.pluginRoot`: the directory bare relative sources resolve under. */
  readonly pluginRoot?: string;
  readonly context: MarketplaceContext;
  /** `load` unless the marketplace is being validated. */
  readonly mode?: ReadMode;
}

/** Reads one raw `plugins[i]` into an entry; never throws, an entry that cannot load is `undefined`. */
export type EntryReader = (raw: unknown, slot: EntrySlot) => EntryReading;

/**
 * The parsed `marketplace.json` as Claude Code loads it: an error in a top-level field or in the
 * name keeps the marketplace from loading; an entry with an error is dropped alone. Without a
 * reader, each entry goes through the entry reader over the plugin source parser (tests of the
 * top level pass a stand-in).
 */
export const readMarketplace = (
  raw: unknown,
  context: MarketplaceContext = {},
  readEntry?: EntryReader,
): MarketplaceResult => call('readMarketplace', raw, context, readEntry);

/** What `claude plugin validate` reports about a `marketplace.json`. */
export const validateMarketplace = (
  raw: unknown,
  context: MarketplaceContext = {},
  readEntry?: EntryReader,
): PluginIssue[] => call('validateMarketplace', raw, context, readEntry);

/** `description`, or `metadata.description` when the top level has none ("Alternate location"). */
export const effectiveDescription = (marketplace: Marketplace): string | undefined =>
  call('effectiveDescription', marketplace);

/** `version`, or `metadata.version` when the top level has none. */
export const effectiveVersion = (marketplace: Marketplace): string | undefined =>
  call('effectiveVersion', marketplace);
