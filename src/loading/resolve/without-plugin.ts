/**
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/without_plugin.rs`,
 * docs/RUST.md).
 */
import type { Locations } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/resolve/without-plugin');

/**
 * Judgment (spec/judgments.md): a handler of no plugin (a settings file's) has no plugin folder
 * and no data folder, so the two plugin variables stay as written instead of naming the folder of
 * a plugin that is not there. The docs show these two only in plugin hooks. Every method
 * delegates, since a host's `Locations` may be a class instance.
 */
export const locationsWithoutPlugin = (locations: Locations): Locations =>
  call('locationsWithoutPlugin', locations);
