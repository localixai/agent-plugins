import type { HookHandlerType } from '../../protocol/hook-config.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/select/event-catalog');

/**
 * What the hooks reference says about each event beyond its input (hooks.md, "Hook lifecycle" and
 * "Prompt-based hooks"): its cadence, the handler types it supports, and the tool calls it skips.
 * Which field its matcher filters on is `matcher-targets.ts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/event_catalog.rs`,
 * docs/RUST.md).
 */

/**
 * "Events fall into three cadences": per session, per turn, and on every tool call. The other
 * events fire at their own moments (a compaction, a file change) and have no cadence of their own.
 */
export type Cadence = 'session' | 'turn' | 'tool-call' | 'other';

export interface EventInfo {
  readonly cadence: Cadence;
  readonly handlerTypes: readonly HookHandlerType[];
  /**
   * Tools whose calls never fire the event: "PreToolUse and PostToolUse, except `EndConversation`
   * calls, which skip both".
   */
  readonly skippedTools: readonly string[];
}

export const EVENT_CATALOG: { readonly [E in HookEvent]: EventInfo } = call('EVENT_CATALOG');

/** Whether `event` runs handlers of this type; any other is skipped. */
export const supportsHandlerType = (event: HookEvent, type: HookHandlerType): boolean =>
  call('supportsHandlerType', event, type);

/** The events that run handlers of this type. */
export const eventsSupporting = (type: HookHandlerType): readonly HookEvent[] =>
  call('eventsSupporting', type);

/** Whether a call to `toolName` fires `event` at all. */
export const firesForTool = (event: HookEvent, toolName: string): boolean =>
  call('firesForTool', event, toolName);
