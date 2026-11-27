/**
 * hooks.md, "Hook handler fields" (the last paragraph under "Matcher patterns"): "When an event
 * fires, its matching hooks run in parallel. If more than 12 of them are prompt or agent hooks, 12
 * of those run at once and the rest start as running ones finish. Each waiting hook's `timeout`
 * starts counting when that hook starts."
 *
 * The timeout of a waiting hook starts when its run starts because the kit computes the limit
 * when it hands the request to the port, which enforces it from then on; a hook that waits for a
 * slot has no request yet.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/concurrency.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';
import type { RegisteredHandler } from '../../contracts.ts';

const call = coreModule('hooks/handlers/remote/concurrency');

export const MAX_MODEL_HOOKS_AT_ONCE: number = call('MAX_MODEL_HOOKS_AT_ONCE');

/**
 * Runs `run` for every handler at once, except that at most `limit` prompt or agent handlers are
 * running at any moment; the rest start, in order, as running ones finish. Results come back in
 * the order of `entries`, so what merges them is the same from run to run.
 */
export const runLimited = <T>(
  entries: readonly RegisteredHandler[],
  run: (entry: RegisteredHandler) => Promise<T>,
  limit: number = MAX_MODEL_HOOKS_AT_ONCE,
): Promise<T[]> => call.async('runLimited', entries, run, limit);
