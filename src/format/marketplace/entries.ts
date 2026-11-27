/**
 * Reading one plugin entry of `marketplace.json` (plugins/marketplace-reference.md, "Plugin
 * entries" and "Validation messages"): an entry with an error is dropped alone, a warning keeps it.
 * `source` is handed to the plugin source parser (`parsePluginSource`), which this reader takes as a
 * parameter so the two are written and tested apart.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/entries.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { EntryReader, EntryReading, EntrySlot } from './marketplace.ts';
import type { PluginSource } from './types.ts';

const call = coreModule('format/marketplace/entries');

/** What a plugin source parser returns: the source, or `undefined` with the issues that say why. */
export interface SourceReading {
  readonly source: PluginSource | undefined;
  readonly issues: readonly PluginIssue[];
}
export type SourceParser = (raw: unknown, slot: EntrySlot) => SourceReading;

/** Reads one raw `plugins[i]`; never throws. */
export const readEntry = (raw: unknown, slot: EntrySlot, parseSource: SourceParser): EntryReading =>
  call('readEntry', raw, slot, parseSource);

/** The entry reader of a marketplace, over the plugin source parser (pass it to `readMarketplace`). */
export const createEntryReader = (parseSource: SourceParser): EntryReader =>
  call('createEntryReader', parseSource);
