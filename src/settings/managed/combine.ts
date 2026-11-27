/**
 * How `managedSourcesBehavior: "merge"` combines the sources, by kind of key (managed-settings.md,
 * "Compose every managed source"; settings-reference.md, `managedSourcesBehavior`). One combiner per
 * kind, one table from key to kind; a key with no row is an "other key" (the value of the highest
 * source that sets it), and a block with no row merges key by key so its lists and locks follow
 * their own rows (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/combine.rs`, docs/RUST.md).
 */
import type { Settings } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/managed/combine');

/** The settings of the sources (highest first) as `"merge"` combines them. */
export const combineSources = (docs: readonly Settings[]): Settings => call('combineSources', docs);
