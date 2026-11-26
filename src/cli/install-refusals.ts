import { coreModule } from '../wasm/bridge.ts';
import type { ConfigOutcome } from './install-config.ts';
import type { CommandOutput } from './result.ts';

const call = coreModule('cli/install-refusals');

/**
 * The refusals of `claude plugin install` before anything is written: bad `--config` values, a dependency closure that cannot be met.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/install_refusals.rs`, docs/RUST.md).
 */

export const configRefusal = (config: ConfigOutcome): CommandOutput =>
  call('configRefusal', config);

export const closureRefusal = (id: string, messages: readonly string[]): CommandOutput =>
  call('closureRefusal', id, messages);
