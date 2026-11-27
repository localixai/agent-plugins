/** Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/login_keys.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { Settings } from '../../protocol/official.ts';
import type { PolicyLayer } from '../layers.ts';

const call = coreModule('settings/managed/login-keys');

/**
 * Which managed source supplies the login keys (managed-settings.md, "Which managed source supplies
 * login keys"; settings-reference.md, `managedSourcesBehavior`). Under either setting the keys come
 * from the machine's own policy: "the MDM policy or managed settings file, whichever is the
 * higher-ranked one carrying a policy key", ahead of remote settings. When that policy sets any
 * policy key, the file below it contributes no login key, even under `"merge"`. The login keys in
 * force: the machine's policy wins, remote settings fill the first-party keys it omits.
 */
export const loginKeysOf = (
  machine: Settings | undefined,
  remote: Settings | undefined,
): Settings => call('loginKeysOf', machine, remote);

/** The settings without any login key. */
export const withoutLoginKeys = (settings: Settings): Settings =>
  call('withoutLoginKeys', settings);

/**
 * `/login` refuses every new cloud gateway sign-in on the machine while the highest managed source on
 * it carries `gatewayInternalNetworks` with a value that cannot be read (managed-settings.md, "Keys
 * that fail closed"): no value says that, so the managed layer records the key as invalid.
 */
export const gatewaySignInRefused = (layer: PolicyLayer): boolean =>
  call('gatewaySignInRefused', layer);

/**
 * The managed tier as a layer: the login keys the highest machine source carried unreadable are
 * recorded as invalid (`PolicyLayer.invalid`), since only that source supplies them.
 */
export const managedLayer = (
  settings: Settings,
  machine: { readonly invalid: readonly string[] } | undefined,
): PolicyLayer => call('managedLayer', settings, machine);
