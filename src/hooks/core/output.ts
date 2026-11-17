import type { CoreHookEvent } from '../../format/core/hooks.ts';
import type { CommandResult } from '../../ports/index.ts';
import { isRecord } from '../../protocol/is-record.ts';
import {
  firstLine,
  NO_STDERR,
  NON_BLOCKING_STATUS,
  TIMED_OUT,
  timedOutDiscarded,
} from '../verdict/notices.ts';
import type { CoreResult } from './types.ts';

/**
 * What a core handler's process came to (core §7.3.2 item 5, §7.3.3): a status other than `0`, a
 * timeout or a process that never started is a failure; on status `0`, no bytes or only JSON
 * whitespace is no opinion, and anything but exactly one JSON object of one of the event's closed
 * output shapes is a failure, `{}` included. Exit status 2 is a failure like any other (Agent
 * Plugins Extended §18.1.2, "Failures": never a block), and plain text is never context.
 */

/** A field of a closed output shape: a nonempty string, or one fixed value. */
type Field = 'text' | { readonly is: string };
type Shape = Readonly<Record<string, Field>>;

const CONTEXT: Shape = { additional_context: 'text' };
const decided = (value: string, reason: boolean): Shape =>
  reason ? { decision: { is: value }, reason: 'text' } : { decision: { is: value } };

/** The output schemas of `schemas/1.1.0/hooks.schema.json`, every one closed. */
const SHAPES: { readonly [E in CoreHookEvent]: readonly Shape[] } = {
  SessionStart: [CONTEXT],
  UserPromptSubmit: [CONTEXT, decided('deny', true)],
  PreToolUse: [decided('allow', false), decided('deny', true)],
  PostToolUse: [CONTEXT],
  Stop: [decided('continue', true)],
};

const fits = (field: Field, value: unknown): boolean =>
  typeof value === 'string' && (field === 'text' ? value !== '' : value === field.is);

function matches(shape: Shape, output: Readonly<Record<string, unknown>>): boolean {
  const keys = Object.keys(output);
  const fields = Object.entries(shape);
  return keys.length === fields.length && fields.every(([key, field]) => fits(field, output[key]));
}

const DECISIONS: ReadonlySet<string> = new Set(['allow', 'deny', 'continue']);
const isDecision = (value: unknown): value is 'allow' | 'deny' | 'continue' =>
  typeof value === 'string' && DECISIONS.has(value);

function resultOf(output: Readonly<Record<string, unknown>>): CoreResult {
  const { additional_context: context, decision, reason } = output;
  if (typeof context === 'string') return { context };
  const chosen = isDecision(decision) ? decision : 'continue';
  return typeof reason === 'string' ? { decision: chosen, reason } : { decision: chosen };
}

/** JSON whitespace only (RFC 8259: space, tab, line feed, carriage return). */
const BLANK = /^[ \t\n\r]*$/;

function parsed(stdout: string): { readonly value: unknown } | undefined {
  try {
    return { value: JSON.parse(stdout) };
  } catch {
    return undefined;
  }
}

export type CoreReading = { readonly result?: CoreResult } | { readonly failure: string };

/**
 * J18.22: a timed-out core handler's cause is the timeout text a Claude Code `command` hook gets on
 * the same event: the discarded output named on `UserPromptSubmit`, `timed out` on the others.
 */
const TIMEOUT_TEXT: { readonly [E in CoreHookEvent]: (timeoutMs: number) => string } = {
  SessionStart: () => TIMED_OUT,
  UserPromptSubmit: timedOutDiscarded,
  PreToolUse: () => TIMED_OUT,
  PostToolUse: () => TIMED_OUT,
  Stop: () => TIMED_OUT,
};

/**
 * The failure of a process that did not exit with status 0, worded as a Claude Code exec-form
 * `command` hook's non-blocking error on the same event (J18.22): the launch error as it is, the
 * timeout text, `Failed with non-blocking status code: <first stderr line>` for any other ending
 * (a process ended by a signal has no status and is worded as one, its stderr first).
 */
function processFailure(
  event: CoreHookEvent,
  run: CommandResult,
  timeoutMs: number,
): string | undefined {
  if (run.startError !== undefined) return run.startError;
  if (run.timedOut) return TIMEOUT_TEXT[event](timeoutMs);
  if (run.exitCode !== 0) return `${NON_BLOCKING_STATUS} ${firstLine(run.stderr) || NO_STDERR}`;
  return undefined;
}

/** Reads one finished run of a core handler for its event. */
export function readCoreRun(
  event: CoreHookEvent,
  run: CommandResult,
  timeoutMs: number,
): CoreReading {
  const failure = processFailure(event, run, timeoutMs);
  if (failure !== undefined) return { failure };
  if (BLANK.test(run.stdout)) return {};
  const json = parsed(run.stdout);
  if (json === undefined) return { failure: 'wrote output that is not one JSON object' };
  const { value } = json;
  if (!isRecord(value) || !SHAPES[event].some((shape) => matches(shape, value))) {
    return { failure: `wrote an object that is not a ${event} result` };
  }
  return { result: resultOf(value) };
}
