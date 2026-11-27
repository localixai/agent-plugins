import type { BackgroundDelivery, HookOutcome } from '../hooks/index.ts';
import type { HookEvent } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The repeat check of async `SessionStart` hooks (hooks.md, "Limitations"; Agent Plugins Extended
 * §7.3.3 item 7): the hook runs every time the event fires, and its output is skipped when its
 * `additionalContext` and `systemMessage` equal what Claude last received from that hook, unless a
 * synchronous `SessionStart` hook added text since. A compaction or `/clear` forgets what each hook
 * gave, so the first run after it delivers again.
 *
 * Judgment (spec/judgments.md): "what Claude last received" is the last delivery of that handler
 * the session handed on, and any text a synchronous `SessionStart` hook adds counts as "other text".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_start_repeats.rs`,
 * docs/RUST.md).
 */
export interface SessionStartRepeats {
  /** The background promises of one dispatch, with the repeats of a `SessionStart`'s emptied. */
  watch(outcome: HookOutcome<HookEvent>, source: unknown): readonly Promise<BackgroundDelivery>[];
}

const call = coreModule('host/session-start-repeats');

export const createSessionStartRepeats = (): SessionStartRepeats =>
  call('createSessionStartRepeats');
