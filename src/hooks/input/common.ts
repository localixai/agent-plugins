import type { BaseHookInput, HookEvent } from '../../protocol/official.ts';

/**
 * The fields every event's input has (hooks.md, "Common input fields"). `scratchpad_dir` is in the
 * docs ("Requires Claude Code v2.1.257 or later") but not in the SDK's `BaseHookInput`, so it is
 * typed here as a documented extension (spec/judgments.md).
 *
 * The core builds these fields (`crates/agent-plugins/src/hooks/input/common.rs`, docs/RUST.md).
 */
export type CommonInput = BaseHookInput & {
  readonly hook_event_name: HookEvent;
  readonly scratchpad_dir?: string;
};
