/**
 * What becomes of a would-be prompt where nobody may answer it. Two facts of the session say so:
 * `cannotPrompt`, a `-p` or SDK run with no permission callback ("Calls needing approval are
 * denied", permission-modes.md), and `permissions.denyInsteadOfPrompting` ("Claude Code denies a
 * tool call that would otherwise wait on a prompt, and the turn continues", settings-reference.md).
 * One rule for both: an `ask` is a denial, every other outcome stands. The setting names its own
 * reason, because its denial carries its own messages.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/unanswered.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { Decision } from '../types.ts';
import type { DecideContext } from './context.ts';

const call = coreModule('rules/decide/unanswered');

export type PromptFacts = Pick<DecideContext, 'cannotPrompt' | 'denyInsteadOfPrompting'>;

/** The decision once the session's prompt facts apply: an `ask` nobody may answer is denied. */
export const settlePrompt = (decision: Decision, facts: PromptFacts): Decision =>
  call('settlePrompt', decision, facts);
