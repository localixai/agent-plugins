/** Implemented by the Rust core (`crates/agent-plugins/src/settings/canonical.rs`, docs/RUST.md). */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('settings/canonical');

/**
 * The lists as one, each entry once, first occurrence kept, so the highest file's entries come
 * first: two entries are one when their canonical text is the same (managed-settings.md, "Split a
 * file-based policy across teams": the lists combine "with duplicates removed"). The one union of
 * the settings layer.
 */
export const unionOf = <E>(lists: readonly (readonly E[])[]): E[] => call('unionOf', lists);
