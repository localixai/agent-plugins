/**
 * Which lock stops a hook of each origin: one registry typed over the origin kinds, so a new kind
 * fails to compile until it says what the locks do to it. `disableAllHooks` is two columns because
 * its reach "depends on which file carries the key" (settings-reference.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/hooks/origin_stops.rs`, docs/RUST.md).
 */
import type { ForceEnabled, PolicyHookSource, Stops } from './origins.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/hooks/origin-stops');

/** Which locks stop a hook of this origin. */
export const stopsOf = (origin: PolicyHookSource, forceEnabled: ForceEnabled): Stops =>
  call('stopsOf', origin, forceEnabled);
