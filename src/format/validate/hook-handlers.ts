/**
 * The hook handler schemas of `claude plugin validate` (oracle 2.1.319): five types, each checked
 * for the fields it needs and the types of the common ones. A handler that fails is reported by
 * type and by field, in the order the validator tests them; anything else in it is left alone.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/hook_handlers.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/validate/hook-handlers');

/** Why a handler cannot load, or `undefined` when it can. */
export const handlerProblem = (handler: unknown): string | undefined =>
  call('handlerProblem', handler);

/** The unquoted-placeholder warning of a handler that loads, if it earns one. */
export const handlerAdvice = (handler: unknown): string | undefined =>
  call('handlerAdvice', handler);
