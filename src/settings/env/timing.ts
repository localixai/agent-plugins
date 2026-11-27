import type { EnvResolution } from './resolve-env.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/env/timing');

/**
 * When Claude Code applies `env` values again (settings-reference.md, "When Claude Code applies `env`
 * values"; env-vars.md, "Set variables in settings files"): in a running session when a saved change
 * alters the merged `env`, and after `/cd` with the new directory's project and local values.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/env/timing.rs`, docs/RUST.md).
 */
export interface EnvChange {
  /** The new and the changed variables: what the running session writes into its environment. */
  readonly set: Readonly<Record<string, string>>;
  /**
   * The variables the new `env` no longer holds. "Removing a variable from the file doesn't unset it
   * in a running session; the removal takes effect the next time you launch `claude`."
   */
  readonly keptUntilRelaunch: readonly string[];
}

/** What a saved change to the `env` of the files does to a running session. */
export const envChange = (
  applied: Readonly<Record<string, string>>,
  next: Readonly<Record<string, string>>,
): EnvChange => call('envChange', applied, next);

/**
 * The session after `/cd` (permissions.md, "Move the session to another directory"; Claude Code
 * v2.1.246 or later): "the new directory's project and local `env` values, on top of the previous
 * directory's". `entered` is the resolution for the new directory's layers; its user, managed and
 * `--settings` values were applied at startup and stay as they are.
 */
export const envAfterCd = (previous: EnvResolution, entered: EnvResolution): EnvResolution =>
  call('envAfterCd', previous, entered);
