import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HookOutcome } from '../outcome.ts';

/**
 * What a blocked prompt leaves behind (hooks.md, "What a blocked prompt leaves behind"): "By
 * default the block message shown to the user ends with `Original prompt:` followed by the
 * submitted text". Printing `suppressOriginalPrompt: true` in `hookSpecificOutput` leaves the text
 * out, "whether the hook blocks with `decision: "block"` or by exiting 2". It changes only the
 * block message: the host still writes the submitted text wherever it keeps the transcript.
 *
 * Judgment (spec/judgments.md): the docs give no layout for the ending, so the label starts a line
 * of its own. A block no one is shown (`to` is not the user) has no message to end.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/blocked_prompt.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/verdict/blocked-prompt');

export const withOriginalPrompt = <E extends HookEvent>(
  outcome: HookOutcome<E>,
  facts: object,
): HookOutcome<E> => call('withOriginalPrompt', outcome, facts);
