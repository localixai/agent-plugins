import { coreModule } from '../../wasm/bridge.ts';
import type { ProfileContext } from './profiles.ts';
import type { CoreToolProfile } from './types.ts';

/**
 * The portable view of a completed call (Agent Plugins Extended §18.2.7; core §7.3.7): the pending
 * profile of the call as it ran, with the result projected from Claude Code's `tool_response` on
 * `PostToolUse` or its `error` on `PostToolUseFailure`. Where the result cannot be given (no exit
 * code, no HTTP status, no `CallToolResult`, no text Claude received), the whole profile is omitted,
 * as core §7.3.7 requires (Judgments J18.16, J18.21).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/core/profile_results.rs`,
 * docs/RUST.md).
 */

const call = coreModule('hooks/core/profile-results');

/** What the call came to: its response, or the failure text Claude received. */
export type Completed =
  | { readonly ok: true; readonly response: unknown }
  | { readonly ok: false; readonly error: string };

/** The profile of a completed call, or `undefined` when none applies or its result is unknown. */
export const completedProfile = (
  toolName: string,
  toolInput: unknown,
  completed: Completed,
  context: ProfileContext,
): CoreToolProfile | undefined => call('completedProfile', toolName, toolInput, completed, context);
