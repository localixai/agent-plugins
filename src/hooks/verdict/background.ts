import type { BackgroundResult } from '../handlers/context.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { BackgroundDelivery } from './types.ts';

/**
 * What a background hook hands over once its process ends (hooks.md, "How async hooks execute"):
 * "Claude Code delivers the `additionalContext` and `systemMessage` fields from the hook's JSON
 * response to Claude on the next conversation turn". The response is validated and "any field whose
 * value has the wrong type, such as a `systemMessage` that isn't a string" is dropped. Nothing else
 * of the response matters: "response fields like `decision`, `permissionDecision`, and `continue`
 * have no effect, because the action they would have controlled has already completed". An
 * `asyncRewake` hook that exited with code 2 "wakes Claude" with its stderr or stdout.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/background.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/verdict/background');

export const deliveryOf = (handlerId: string, result: BackgroundResult): BackgroundDelivery =>
  call('deliveryOf', handlerId, result);
