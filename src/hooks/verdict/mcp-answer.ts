import type { HookEvent } from '../../protocol/official.ts';
import type { McpRun } from '../handlers/remote/types.ts';
import { EXIT2_RULES } from './exit2.ts';
import { interpretExited } from './exited.ts';
import { failureVerdict, onFailureBlocks, refusedVerdict, type Failure } from './on-failure.ts';
import { readBody } from './stdout-kind.ts';
import type { Subject } from './subject.ts';
import type { HandlerVerdict } from './types.ts';

/**
 * An `mcp_tool` hook's answer (hooks.md, "How the tool's result is read"; Agent Plugins Extended
 * §7.3.5 items 2–4, §7.4.3): the text is read as command-hook stdout; `isError: true`, a server that
 * was not connected and an answer the client could not read are failures, which `onFailure:
 * "block"` (2.1.328) turns into a block.
 */

/** The events where an unreadable answer refuses what the hook was checking, whatever `onFailure` says. */
const REFUSES_UNREADABLE: ReadonlySet<HookEvent> = new Set<HookEvent>(['PreToolUse']);

/** The fields of a hook's JSON output (hooks.md, "JSON output"), `reason` excepted (J7.212). */
const OUTPUT_FIELDS: ReadonlySet<string> = new Set([
  'continue',
  'stopReason',
  'suppressOutput',
  'systemMessage',
  'terminalSequence',
  'decision',
  'hookSpecificOutput',
]);

/**
 * J7.212: with `"block"` on an event that can block, "a non-empty answer also counts as a failure
 * unless it is `{}` or JSON that sets a hook output field other than `reason`".
 */
function answerFails(subject: Subject, text: string): boolean {
  if (!onFailureBlocks(subject) || !EXIT2_RULES[subject.event].canBlock) return false;
  const body = readBody(text);
  if (body.empty) return false;
  if (body.json === undefined) return true;
  const keys = Object.keys(body.json);
  return keys.length > 0 && !keys.some((key) => OUTPUT_FIELDS.has(key));
}

const toolError = (text: string): Failure => ({ cause: 'tool-error', text, detail: text });

const NO_OUTPUT_FIELD =
  'the answer is not empty, not {} and sets no hook output field, which counts as a failure under onFailure "block"';

export function interpretMcp(subject: Subject, run: McpRun): HandlerVerdict {
  if (run.unreadable !== undefined) {
    return REFUSES_UNREADABLE.has(subject.event)
      ? refusedVerdict(subject, toolError(run.text))
      : failureVerdict(subject, toolError(run.text));
  }
  if (run.isError) return failureVerdict(subject, toolError(run.text));
  if (answerFails(subject, run.text)) {
    return failureVerdict(subject, {
      cause: 'invalid-output',
      text: NO_OUTPUT_FIELD,
      detail: run.text,
    });
  }
  return interpretExited(subject, { exitCode: 0, stdout: run.text, stderr: '' });
}
