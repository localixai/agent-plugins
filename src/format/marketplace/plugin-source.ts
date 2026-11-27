import { coreModule } from '../../wasm/bridge.ts';
import type { EntrySlot } from './marketplace.ts';
import type { SourceReading } from './entries.ts';

/**
 * Reading a plugin entry's `source` (plugins/marketplace-reference.md, "Plugin sources", "Relative
 * path plugin source", "Bare names under pluginRoot", "Validation messages"): a relative path
 * string, or an object whose own `source` key names one of the other six types. It is the
 * `SourceParser` the entry reader is given (`createEntryReader(parsePluginSource)`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/plugin_source.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/marketplace/plugin-source');

/** The plugin source of `plugins[i]`: the source, or `undefined` with the issues that say why. */
export const parsePluginSource = (raw: unknown, slot: EntrySlot): SourceReading =>
  call('parsePluginSource', raw, slot);
