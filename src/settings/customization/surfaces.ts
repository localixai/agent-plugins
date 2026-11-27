/**
 * The kinds of customization `strictPluginOnlyCustomization` locks.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/customization/surfaces.rs`,
 * docs/RUST.md).
 */
import type { PluginOnlySurface } from '../../protocol/settings-keys.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/customization/surfaces');

/** The kinds of customization the lock covers, and how many names in the list were no kind. */
export interface SurfaceLocks {
  readonly locked: ReadonlySet<PluginOnlySurface>;
  /** Names this version does not recognize: they lock nothing, and a status note counts them. */
  readonly unknownNames: number;
}

/**
 * What the value of `strictPluginOnlyCustomization` locks (settings-reference.md,
 * `strictPluginOnlyCustomization`): `true` locks all four kinds, an array locks the kinds it names,
 * and anything unset locks nothing. A name no kind has "locks nothing; a status note counts such
 * entries" (managed-settings.md, "Invalid entries in managed settings"), the same page that treats a
 * value "neither a boolean nor an array" as `true`. `false` is the SDK type's "explicit no-op"
 * (spec/judgments.md).
 */
export const readSurfaceLocks = (value: unknown): SurfaceLocks => call('readSurfaceLocks', value);
