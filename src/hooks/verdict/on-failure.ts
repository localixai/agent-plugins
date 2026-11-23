import type { HookHandlerType } from '../../protocol/hook-config.ts';
import { routedBlock, shownWith } from './route.ts';
import type { HandlerVerdict, BlockInfo, BlockVia } from './types.ts';
import { EXIT2_RULES } from './exit2.ts';
import {
  failedBlocking,
  handlerLabel,
  hookError,
  hookName,
  timedOutBlocking,
  timedOutBlocksSwitch,
  windowsBlocking,
} from './notices.ts';
import type { Subject } from './subject.ts';

/**
 * A hook that fails (hooks.md, "Block the action when a hook fails"): "Each of these counts as a
 * failure": a command hook that can't start, an exit code other than 0 or 2, an HTTP error, a
 * timeout, invalid output. Without `onFailure: "block"` a failure is a non-blocking error and "the
 * action goes ahead"; with it "a failure does what exit code 2 does on that event, except on
 * `PermissionRequest`, where it denies the request".
 */

export const FAILURE_CAUSES = [
  'cannot-start',
  'other-exit',
  'http-error',
  'timeout',
  'invalid-output',
  /**
   * An `mcp_tool` hook's tool answered `isError: true`, its server was not connected, or its answer
   * could not be read; with `onFailure: "block"` (2.1.328) a failure (§7.4.3, "MCP failure").
   */
  'tool-error',
  /** A prompt or agent hook's model gave no verdict (an agent hook "ended without a decision"). */
  'no-decision',
] as const;
export type FailureCause = (typeof FAILURE_CAUSES)[number];

export interface Failure {
  readonly cause: FailureCause;
  /** What the notice of a non-blocking error carries. */
  readonly text: string;
  /** The hook's own error output, which follows `failed; blocking because ...`. */
  readonly detail: string;
}

/** The `onFailure` of a `command`, `http` or `mcp_tool` hook; the other types have no such field. */
const onFailureOf = (subject: Subject): 'continue' | 'block' | undefined => {
  const { handler } = subject.entry;
  return 'onFailure' in handler ? handler.onFailure : undefined;
};

/**
 * "On Windows without Git Bash, a shell-form `PreToolUse` or `PermissionRequest` command hook that
 * fails with `onFailure` unset can instead block the tool call": without Git Bash "any exit code
 * other than 0 or 2 from that hook counts as a failure". A hook in exec form (`args` set) is not
 * one: "Claude Code then starts the program with no shell".
 */
const WINDOWS_GATES: ReadonlySet<string> = new Set(['PreToolUse', 'PermissionRequest']);

const isWindowsGate = (subject: Subject): boolean => {
  const { handler } = subject.entry;
  return (
    subject.gitBashMissing &&
    WINDOWS_GATES.has(subject.event) &&
    'command' in handler &&
    handler.args === undefined &&
    onFailureOf(subject) === undefined
  );
};

/**
 * "Background command hooks: command hooks that set `async` or `asyncRewake`": `onFailure` has no
 * effect on them.
 */
const runsInBackground = (subject: Subject): boolean => {
  const { handler } = subject.entry;
  // `asyncRewake` "implies async" (the SDK type): either field alone makes the hook a background one.
  return 'command' in handler && (handler.async === true || handler.asyncRewake === true);
};

/**
 * `onFailure: "block"` is set and takes effect on this hook: not on the events whose exit 2 keeps
 * Claude working (`Stop`, `SubagentStop`, `TaskCompleted`, `TeammateIdle`) and not on background hooks.
 */
export const onFailureBlocks = (subject: Subject): boolean =>
  (onFailureOf(subject) === 'block' || isWindowsGate(subject)) &&
  EXIT2_RULES[subject.event].ignoresOnFailure !== true &&
  !runsInBackground(subject);

const viaOf = (failure: Failure): BlockVia => (failure.cause === 'timeout' ? 'timeout' : 'failure');

/** The text of the hook error line. */
const lineOf = (subject: Subject, reason: string) => ({
  reason,
  message: hookError(
    hookName(subject.event, subject.toolName),
    handlerLabel(subject.entry.handler),
    reason,
  ),
});

const blockInfo = (
  subject: Subject,
  failure: Failure,
  reason: string,
  conditional: boolean,
): BlockInfo =>
  routedBlock(
    subject.event,
    viaOf(failure),
    lineOf(subject, reason),
    conditional ? 'worktree-directory-remains' : undefined,
  );

/** The text a failure that blocks carries: `failed; blocking because onFailure is "block"` and the detail. */
function blockingText(subject: Subject, failure: Failure): string {
  if (isWindowsGate(subject)) return windowsBlocking(failure.detail);
  return failure.cause === 'timeout'
    ? timedOutBlocking(failure.detail)
    : failedBlocking(failure.detail);
}

const base = (subject: Subject) => ({
  handler: subject.entry,
  succeeded: false,
  issues: [],
});

/** "On PreModelSwitch, a hook canceled at its timeout blocks the model switch." Judgment: an explicit `onFailure: "continue"` lets it go ahead. */
const blocksSwitch = (subject: Subject, failure: Failure): boolean =>
  EXIT2_RULES[subject.event].timeoutBlocks === true &&
  failure.cause === 'timeout' &&
  onFailureOf(subject) !== 'continue';

/** `onFailure: "block"` where the event can be blocked, or where it denies (`PermissionRequest`). */
function blocksByOnFailure(subject: Subject): boolean {
  const rule = EXIT2_RULES[subject.event];
  return (
    onFailureBlocks(subject) &&
    (rule.canBlock || rule.failureDenies === true) &&
    rule.cannotBlockWhen?.(subject.facts) !== true
  );
}

/** The kind of block a failure comes to on this event, if any. */
function blockFor(subject: Subject, failure: Failure): BlockInfo | undefined {
  const rule = EXIT2_RULES[subject.event];
  const conditional = rule.blocksUnlessDirectoryGone === true;
  // "Except on events with their own failure contract in the per-event table, such as WorktreeCreate".
  if (rule.anyNonZeroBlocks === true) {
    return blockInfo(subject, failure, failure.detail || failure.text, conditional);
  }
  if (blocksByOnFailure(subject)) {
    return blockInfo(subject, failure, blockingText(subject, failure), conditional);
  }
  return blocksSwitch(subject, failure)
    ? blockInfo(subject, failure, timedOutBlocksSwitch(handlerLabel(subject.entry.handler)), false)
    : undefined;
}

/**
 * `onFailure: "block"` on an event where exit 2 only shows stderr (`PostToolUse`): the failure does
 * what exit 2 does there, which is the channel of the table's row.
 */
function failureAsExit2Feedback(subject: Subject, failure: Failure): HandlerVerdict | undefined {
  const rule = EXIT2_RULES[subject.event];
  if (!onFailureBlocks(subject) || rule.canBlock || rule.failureDenies === true) return undefined;
  const { message } = lineOf(subject, blockingText(subject, failure));
  const handlerId = subject.entry.id;
  if (rule.stderr === 'claude') {
    return {
      ...base(subject),
      status: 'non-blocking-error',
      feedback: { handlerId, to: 'claude', text: message },
    };
  }
  if (rule.stderr === 'user') {
    return { ...base(subject), status: 'non-blocking-error', notice: { handlerId, message } };
  }
  return { ...base(subject), status: 'non-blocking-error' };
}

/**
 * Code Mode (code-mode.md, "Tool call refused by a hook"; §7.6.11.4, J7.213): a `PreToolUse` hook
 * that gives no decision on a call from code, because it timed out or an agent hook ended without
 * one, refuses the call when `onFailure` is unset. Other failures let it run (2.1.329).
 */
const DID_NOT_FINISH = 'the hook did not finish';

const REFUSES_UNFINISHED: ReadonlySet<string> = new Set(['PreToolUse']);

/** "an agent hook ended without a decision": the type whose missing verdict counts as unfinished. */
const ENDS_WITHOUT_DECISION: { readonly [T in HookHandlerType]: boolean } = {
  command: false,
  http: false,
  mcp_tool: false,
  prompt: false,
  agent: true,
};

const unfinished = (subject: Subject, failure: Failure): boolean =>
  failure.cause === 'timeout' ||
  (failure.cause === 'no-decision' && ENDS_WITHOUT_DECISION[subject.entry.handler.type]);

const refusesFromCode = (subject: Subject, failure: Failure): boolean =>
  subject.fromCode === true &&
  REFUSES_UNFINISHED.has(subject.event) &&
  onFailureOf(subject) === undefined &&
  unfinished(subject, failure);

/** What a failed run comes to. */
export function failureVerdict(subject: Subject, failure: Failure): HandlerVerdict {
  if (refusesFromCode(subject, failure)) {
    return refusedVerdict(subject, { ...failure, text: DID_NOT_FINISH });
  }
  const block = blockFor(subject, failure);
  if (block !== undefined) {
    return { ...base(subject), status: 'blocked', block, ...shownWith(subject.entry.id, block) };
  }
  const feedback = failureAsExit2Feedback(subject, failure);
  if (feedback !== undefined) return feedback;
  const { message } = lineOf(subject, failure.text);
  const status = failure.cause === 'timeout' ? 'timed-out' : 'non-blocking-error';
  if (EXIT2_RULES[subject.event].noFailureNotice === true) return { ...base(subject), status };
  return { ...base(subject), status, notice: { handlerId: subject.entry.id, message } };
}

/**
 * A verdict that refuses the action whatever `onFailure` says: an `mcp_tool` answer the client could
 * not read refuses the `PreToolUse` call it was checking (hooks.md, "How the tool's result is read").
 */
export function refusedVerdict(subject: Subject, failure: Failure): HandlerVerdict {
  const block = blockInfo(subject, failure, failure.text, false);
  return { ...base(subject), status: 'blocked', block, ...shownWith(subject.entry.id, block) };
}
