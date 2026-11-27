import type { HookEvent } from '../../protocol/official.ts';

/**
 * `statusMessage` (hooks.md, "Common fields"): "Custom spinner message displayed while the hook
 * runs". Showing a spinner is the host's job; the kit tells it when a hook starts and with what
 * text. The core announces a start (`crates/agent-plugins/src/hooks/handlers/status.rs`); these
 * are the shapes the host's observer is called with.
 */

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
