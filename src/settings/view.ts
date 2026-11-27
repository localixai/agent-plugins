import type { SettingsKey, ValueOf } from './general/keys.ts';
import type { KeyHolder, PolicyLayer } from './layers.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Typed questions over the ordered layers, the one thing every area resolver asks (docs/API.md,
 * "Settings"). A resolver never walks the layers itself, so none of them repeats the precedence
 * order, the scope lines or the merge rules.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/view.rs`, docs/RUST.md).
 */
export interface PolicyView {
  /** The layers, highest precedence first. */
  readonly layers: readonly PolicyLayer[];
  /** The key as the files combine it (`undefined` when no file sets it). */
  get<K extends SettingsKey>(key: K): ValueOf<K> | undefined;
  /** The highest layer that sets the key. */
  winner<K extends SettingsKey>(key: K): KeyHolder<K> | undefined;
  /** Every layer that sets the key, highest first: what a key with its own rule walks. */
  eachFromHighest<K extends SettingsKey>(key: K): readonly KeyHolder<K>[];
}

const call = coreModule('settings/view');

export const createPolicyView = (given: readonly PolicyLayer[]): PolicyView =>
  call('createPolicyView', given);
