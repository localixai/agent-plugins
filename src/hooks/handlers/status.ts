import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { RegisteredHandler } from '../contracts.ts';

/**
 * `statusMessage` (hooks.md, "Common fields"): "Custom spinner message displayed while the hook
 * runs". Showing a spinner is the host's job; the kit tells it when a hook starts and with what
 * text.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/status.rs`, docs/RUST.md).
 */
const call = coreModule('hooks/handlers/status');

/** What the host is told when a hook starts. */
export interface HookStarted {
  readonly event: HookEvent;
  readonly handlerId: string;
  /** Absent when the handler has none: the host shows its own label. */
  readonly statusMessage?: string;
}

/** The part of the host's hook observer the run stage drives; the other callbacks are the engine's. */
export interface StartObserver {
  started?(info: HookStarted): void;
}

/**
 * Calls one of the host's observer callbacks. A callback that throws is the host's fault and tells
 * the host nothing it can use mid-dispatch: it neither stops the hook nor rejects the dispatch, so
 * a sibling hook's block still reaches the outcome (judgment, spec/judgments.md).
 */
export const observe = (callback: () => void): void => call('observe', callback);

export const announceStart = (
  observer: StartObserver | undefined,
  entry: RegisteredHandler,
): void => call('announceStart', observer, entry);
