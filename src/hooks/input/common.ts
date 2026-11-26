import type { PluginIssue } from '../../format/issues.ts';
import type { BaseHookInput, HookEvent } from '../../protocol/official.ts';
import type { HookSession } from '../contracts.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/input/common');

/**
 * The fields every event's input has (hooks.md, "Common input fields"). `scratchpad_dir` is in the
 * docs ("Requires Claude Code v2.1.257 or later") but not in the SDK's `BaseHookInput`, so it is
 * typed here as a documented extension (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/input/common.rs`, docs/RUST.md).
 */
export type CommonInput = BaseHookInput & {
  readonly hook_event_name: HookEvent;
  readonly scratchpad_dir?: string;
};

/**
 * The common fields of `event`'s input from what the host knows of the session, in the order the
 * docs' example lists them. A field the host has no value for is absent from the object, as the
 * docs say of `prompt_id` ("Absent until the first user input") and `scratchpad_dir` ("Absent when
 * the session has no scratchpad").
 */
export const commonInput = (
  event: HookEvent,
  session: HookSession,
): { readonly fields: CommonInput; readonly issues: readonly PluginIssue[] } =>
  call('commonInput', event, session);
