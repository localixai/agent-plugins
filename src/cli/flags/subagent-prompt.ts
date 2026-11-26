import { coreModule } from '../../wasm/bridge.ts';
import type { Contribution, FlagContext, FlagFacts } from './types.ts';

const call = coreModule('cli/flags/subagent-prompt');

/**
 * `--append-subagent-system-prompt`, `--append-subagent-system-prompt-file` and
 * `--forward-subagent-text` (cli-reference.md, "CLI flags"; errors.md, "Conflict between a
 * system-prompt flag and its file form"; headless.md, "Follow subagent messages"). The first two
 * append text to every subagent's system prompt and "only apply in non-interactive mode with `-p`";
 * the two "can't be combined". The last forwards subagent text and thinking blocks and "requires
 * `--print` and `--output-format stream-json`"; `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1` "enables the
 * same behavior" and, "unlike the flag", "is ignored" outside that mode (env-vars.md). What a flag
 * does when its requirement is missing is not stated: it has no effect (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/subagent-prompt.rs`, docs/RUST.md).
 */

/** Both forms give a refusal; the inline form reports it, so the file form stays silent when both are given. */
export const appendSubagentPrompt = (text: string, context: FlagContext): Contribution =>
  call('appendSubagentPrompt', text, context);

export const appendSubagentPromptFile = (path: string, context: FlagContext): Contribution =>
  call('appendSubagentPromptFile', path, context);

/** The variable alone: ignored, without a warning, outside `--print` with `stream-json`. */
export const forwardSubagentTextVariable = (facts: FlagFacts): Contribution =>
  call('forwardSubagentTextVariable', facts);

export const forwardSubagentText = (_value: true, context: FlagContext): Contribution =>
  call('forwardSubagentText', _value, context);
