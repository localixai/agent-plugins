import { coreModule } from '../../wasm/bridge.ts';
import type { RegisteredHandler } from '../contracts.ts';

/**
 * The `once` rule (hooks.md, "Common fields"). The claims of a session's `once` handlers (held
 * while a dispatch runs one, spent after its first successful run) are the core's, by the serial
 * each registration has (docs/RUST.md, "The host session").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/once.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/select/once');

/**
 * Whether the session removes this handler after the run: `once` is set, the source honors it, and
 * the run succeeded. "A run that fails, blocks with exit code 2, or times out leaves the hook in
 * place, so it runs again on the next matching event."
 */
export const consumedByRun = (entry: RegisteredHandler, succeeded: boolean): boolean =>
  call('consumedByRun', entry, succeeded);
