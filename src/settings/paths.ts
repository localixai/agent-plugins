/** Implemented by the Rust core (`crates/agent-plugins/src/settings/paths.rs`, docs/RUST.md). */
import { coreModule } from '../wasm/bridge.ts';
import type { Settings } from '../protocol/official.ts';

const call = coreModule('settings/paths');

/**
 * A key inside a block of a settings object, the way the docs name it: dotted
 * (`sandbox.network.allowedDomains`) or as its names down the blocks.
 */
export type SettingsPath = string | readonly string[];

/** The value at `path`, whatever the blocks on the way; `undefined` when a block is missing. */
export const valueAt = (settings: Settings, path: SettingsPath): unknown =>
  call('valueAt', settings, path);

/**
 * The settings with `value` at `path`, the blocks on the way made; with `value` `undefined` the key
 * is removed, and so is a block the removal leaves empty.
 */
export const withValueAt = (settings: Settings, path: SettingsPath, value: unknown): Settings =>
  call('withValueAt', settings, path, value);
