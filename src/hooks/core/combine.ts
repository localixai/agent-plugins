import type { CoreHookEvent } from '../../format/core/hooks.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { HookEvent } from '../../protocol/official.ts';
import type { HandlerRun, HookOutcome } from '../outcome.ts';
import { withOriginalPrompt } from '../verdict/blocked-prompt.ts';
import { hookError, hookName } from '../verdict/notices.ts';
import { routedBlock } from '../verdict/route.ts';
import type { HookNotice } from '../verdict/types.ts';
import type { CoreOccurrence, CoreRun } from './types.ts';

/**
 * One occurrence's outcome with its core side added (Agent Plugins Extended §18.1.2, "Combining
 * results"): Claude Code's outcome is what the engine made of its handlers, unchanged in what it
 * says of them (principle 1); the core results join it by the event's rule. A failed core handler
 * contributes nothing but its notice (J18.4). Every Claude Code handler precedes every core one in
 * handler order, and core handlers follow the order their packages loaded (J18.3).
 */

export interface CoreSide {
  readonly occurrence: CoreOccurrence;
  readonly runs: readonly CoreRun[];
  /** Every successful `additional_context`, already capped and with its reminder tags escaped. */
  readonly contexts: readonly string[];
  readonly issues: readonly PluginIssue[];
}

type Outcome = HookOutcome<HookEvent>;
type Combine = (outcome: Outcome, side: CoreSide, facts: object) => Outcome;

const decisionsOf = (side: CoreSide, decision: 'allow' | 'deny' | 'continue') =>
  side.runs.flatMap((run) =>
    run.result !== undefined && 'decision' in run.result && run.result.decision === decision
      ? [{ handlerId: run.handlerId, reason: run.result.reason ?? '' }]
      : [],
  );

/** J18.22: the reasons in handler order, an absent or empty one left out, one newline between. */
const joinedReasons = (found: readonly { readonly reason: string }[]): string =>
  found
    .map((one) => one.reason)
    .filter((reason) => reason !== '')
    .join('\n');

const withContexts: Combine = (outcome, side) => ({
  ...outcome,
  additionalContext: [...outcome.additionalContext, ...side.contexts],
});

/**
 * `UserPromptSubmit`: blocked when either side blocks; a blocked prompt delivers no context from
 * either side (core §7.3.5). A Claude Code block keeps its own message; else the core reasons are
 * the message, ending with the prompt (Claude Code's `suppressOriginalPrompt` belongs to a blocking
 * Claude Code handler only, §7.5.8).
 */
const userPromptSubmit: Combine = (outcome, side, facts) => {
  if (!outcome.proceed) return { ...outcome, additionalContext: [] };
  const denials = decisionsOf(side, 'deny');
  const first = denials[0];
  if (first === undefined) return withContexts(outcome, side, facts);
  const reason = joinedReasons(denials);
  const { suppressOriginalPrompt: _blockingOnly, ...effects } = outcome.effects;
  const blocked: Outcome = {
    ...outcome,
    proceed: false,
    block: {
      ...routedBlock('UserPromptSubmit', 'decision', { reason, message: reason }),
      by: first.handlerId,
    },
    additionalContext: [],
    effects,
  };
  return { ...withOriginalPrompt(blocked, facts), effects: outcome.effects };
};

/**
 * `PreToolUse`: `deny` > `defer` > `ask` > `allow` > none across both sides. A Claude Code denial
 * (an exit 2 included) keeps its reason; a core denial denies with the core reasons and runs
 * nothing, so no rewrite applies; a core `allow` is an `allow` with no `updatedInput`, and wins
 * only over no decision at all.
 */
const preToolUse: Combine = (outcome, side) => {
  const own = outcome.block === undefined ? outcome.effects.permissionDecision : 'deny';
  if (own === 'deny') return outcome;
  const denials = decisionsOf(side, 'deny');
  const first = denials[0];
  if (first !== undefined) {
    const reason = joinedReasons(denials);
    const { updatedInput: _runsNothing, ...effects } = outcome.effects;
    return {
      ...outcome,
      proceed: false,
      block: {
        ...routedBlock('PreToolUse', 'decision', { reason, message: reason }),
        by: first.handlerId,
      },
      effects: { ...effects, permissionDecision: 'deny', permissionDecisionReason: reason },
    };
  }
  const allows = decisionsOf(side, 'allow').length > 0;
  return allows && own === undefined
    ? { ...outcome, effects: { ...outcome.effects, permissionDecision: 'allow' } }
    : outcome;
};

const appended = (first: string, more: string): string =>
  first === '' || more === '' ? first + more : `${first}\n${more}`;

/**
 * `Stop`: one continuation when either side asks, unless a Claude Code handler returned `continue:
 * false` (J18.9). It carries every reason: a Claude Code block's own, then the core reasons. The
 * cap counts it once, after this (the session applies it to the combined outcome).
 */
const stop: Combine = (outcome, side) => {
  const continues = decisionsOf(side, 'continue');
  const first = continues[0];
  if (first === undefined || !outcome.continue) return outcome;
  const reason = joinedReasons(continues);
  const { block } = outcome;
  if (block !== undefined) {
    return {
      ...outcome,
      block: {
        ...block,
        reason: appended(block.reason, reason),
        message: appended(block.message, reason),
      },
    };
  }
  return {
    ...outcome,
    proceed: false,
    block: { ...routedBlock('Stop', 'decision', { reason, message: reason }), by: first.handlerId },
  };
};

const COMBINE: { readonly [E in CoreHookEvent]: Combine } = {
  SessionStart: withContexts,
  UserPromptSubmit: userPromptSubmit,
  PreToolUse: preToolUse,
  PostToolUse: withContexts,
  Stop: stop,
};

/** A core run as the outcome's run list records it. */
const runOf = (run: CoreRun): HandlerRun => ({
  handlerId: run.handlerId,
  handler: run.label,
  plugin: run.pluginId,
  ending: run.timedOut === true ? 'timed-out' : 'finished',
  ...(run.exitCode === undefined ? {} : { exitCode: run.exitCode }),
  ...(run.timedOut === true ? { timeoutMs: run.timeoutMs } : {}),
  status:
    run.failure === undefined
      ? 'success'
      : run.timedOut === true
        ? 'timed-out'
        : 'non-blocking-error',
});

/** J18.4: a failed core handler is reported as a non-blocking hook error, `<hook name> hook error`. */
const noticesOf = (side: CoreSide): HookNotice[] =>
  side.runs.flatMap((run) =>
    run.failure === undefined
      ? []
      : [
          {
            handlerId: run.handlerId,
            message: hookError(
              hookName(side.occurrence.event, side.occurrence.toolName),
              run.label,
              run.failure,
            ),
          },
        ],
  );

/** The outcome of the occurrence with the core side combined into it. */
export function combineCore<E extends HookEvent>(
  outcome: HookOutcome<E>,
  side: CoreSide,
  facts: object,
): HookOutcome<E> {
  const combined = COMBINE[side.occurrence.event](outcome, side, facts);
  return {
    ...outcome,
    proceed: combined.proceed,
    ...(combined.block === undefined ? {} : { block: combined.block }),
    additionalContext: combined.additionalContext,
    effects: combined.effects,
    notices: [...outcome.notices, ...noticesOf(side)],
    runs: [...outcome.runs, ...side.runs.map(runOf)],
    issues: [...outcome.issues, ...side.issues],
  };
}
