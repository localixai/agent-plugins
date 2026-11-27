/**
 * The numbers a tool reads from the session's environment.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/env_number.rs`,
 * docs/RUST.md).
 */
import type { Environment } from '../../../ports/environment.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/env-number');

/**
 * The value of a variable that is a positive integer; any other text counts as unset
 * (spec/judgments.md: the docs name no behaviour for a variable that is not a number).
 */
export const positiveInteger = (env: Pick<Environment, 'get'>, name: string): number | undefined =>
  call('positiveInteger', env, name);
