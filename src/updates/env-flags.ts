/**
 * The variables that stop updates and traffic, read through the `Environment` port (docs/
 * ARCHITECTURE.md, L3). env-vars.md: `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is "any non-empty
 * value"; the others are "Set to `1`".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/updates/env_flags.rs`, docs/RUST.md).
 */
import type { Environment } from '../ports/environment.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('updates/env-flags');

export const DISABLE_NONESSENTIAL_TRAFFIC: 'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC' = call(
  'DISABLE_NONESSENTIAL_TRAFFIC',
);

/** Whether the variable holds any non-empty value. */
export const isSet = (env: Environment, name: string): boolean => call('isSet', env, name);

/**
 * Whether a "set to `1`" variable is on. The docs say `1`; its table reads `DEBUG` as on for `1`,
 * `true`, `yes` and `on`, and the kit reads these four spellings, in any case, the same way
 * (spec/judgments.md). Anything else, `0` and the empty string included, is off.
 */
export const isOn = (env: Environment, name: string): boolean => call('isOn', env, name);
