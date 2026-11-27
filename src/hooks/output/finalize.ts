import type { HookEvent } from '../../protocol/official.ts';
import type { BlockInfo } from '../verdict/types.ts';
import type { EventEffects } from './effects.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * A block that is not a JSON decision (an exit-2 hook, a failure) holds the action back, so the
 * effects the merge computed from the other hooks must agree with it: "precedence is `deny` >
 * `defer` > `ask` > `allow`", "a decline from one of them overrides an `accept` or `cancel` from
 * another" (hooks.md, "PreToolUse decision control", "Other ways to decline an elicitation").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/finalize.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/output/finalize');

export const finalizeEffects = (
  event: HookEvent,
  effects: EventEffects,
  block: BlockInfo | undefined,
): EventEffects => call('finalizeEffects', event, effects, block);
