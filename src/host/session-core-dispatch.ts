import type { HookOutcome, HookSourceKind, StopBlockCap } from '../hooks/index.ts';
import { combineCore } from '../hooks/core/combine.ts';
import { coreOccurrence, needsTurnOrigin } from '../hooks/core/input.ts';
import type { CoreCallFacts, CoreOccurrence } from '../hooks/core/types.ts';
import type { HookEvent } from '../protocol/official.ts';
import { runCoreSide, selectedCoreHandlers, type CoreHandlerUse } from './session-core-hooks.ts';

/**
 * One occurrence for both forms (Agent Plugins Extended §18.1.2, "When each side runs"): with no
 * core handler on it the dispatch is Claude Code's alone, exactly as without core packages
 * (principle 1). `PreToolUse` runs Claude Code's handlers first and the core's on the input the call
 * would now run with, a denial included (J18.1); every other shared event runs both sides together
 * and waits for both (J18.2). On `Stop` the cap counts the combined continuation once (J18.9).
 */

/** How a session runs its Claude Code side: with the Stop cap applied, or without (the caller applies it). */
export type ClaudeSide<E extends HookEvent> = (capped: boolean) => Promise<HookOutcome<E>>;

export interface CoreDispatch<E extends HookEvent> {
  readonly event: E;
  readonly facts: object;
  readonly use: CoreHandlerUse;
  readonly context: { readonly cwd: string; readonly subagent: boolean };
  readonly core: CoreCallFacts | undefined;
  readonly sourcesOnly: readonly HookSourceKind[] | undefined;
  readonly stopCap: StopBlockCap;
  readonly signal: AbortSignal;
}

/** The decisions under which no `updatedInput` applies (§7.5.11; the core's `hooks/output/merge_gates.rs`). */
const KEEPS_INPUT: ReadonlySet<string | undefined> = new Set([undefined, 'allow', 'ask']);

/** The input the call would run with after Claude Code's handlers (J18.1). */
function effectiveFacts(outcome: HookOutcome<HookEvent>, facts: object): object {
  const { permissionDecision, updatedInput } = outcome.effects;
  const applies = outcome.block === undefined && KEEPS_INPUT.has(permissionDecision);
  return applies && updatedInput !== undefined ? { ...facts, tool_input: updatedInput } : facts;
}

function occurrenceOf<E extends HookEvent>(request: CoreDispatch<E>, facts: object) {
  return coreOccurrence(request.event, facts, {
    sessionId: request.use.session.sessionId,
    cwd: request.context.cwd,
    subagent: request.context.subagent,
    facts: request.core ?? {},
  });
}

/**
 * J18.7: a `system` turn holds peer and channel messages, which fire core `UserPromptSubmit`, and
 * task notifications and automatic continuations, which do not. Only the host knows which, so with
 * a core handler to run the dispatch refuses one without `turnOrigin` rather than guess.
 */
function requireTurnOrigin<E extends HookEvent>(request: CoreDispatch<E>): void {
  if (!needsTurnOrigin(request.event, request.facts, request.core ?? {})) return;
  if (selectedCoreHandlers(request.use, 'UserPromptSubmit').length === 0) return;
  throw new Error(
    'dispatch UserPromptSubmit: a turn whose source is "system" needs SessionDispatchOptions.core.turnOrigin ("external" or "client") when a core package handles UserPromptSubmit (Agent Plugins Extended §18.1.2, J18.7)',
  );
}

/** Whether core handlers take part: a dispatch limited to sources without plugins has none. */
function firstOccurrence<E extends HookEvent>(
  request: CoreDispatch<E>,
): CoreOccurrence | undefined {
  if (request.sourcesOnly !== undefined && !request.sourcesOnly.includes('plugin'))
    return undefined;
  requireTurnOrigin(request);
  const occurrence = occurrenceOf(request, request.facts);
  if (occurrence === undefined) return undefined;
  return selectedCoreHandlers(request.use, occurrence.event).length === 0 ? undefined : occurrence;
}

export async function dispatchBothForms<E extends HookEvent>(
  request: CoreDispatch<E>,
  claude: ClaudeSide<E>,
): Promise<HookOutcome<E>> {
  const first = firstOccurrence(request);
  if (first === undefined) return claude(true);
  const { facts, signal, use } = request;
  if (first.event === 'PreToolUse') {
    const outcome = await claude(true);
    const occurrence = occurrenceOf(request, effectiveFacts(outcome, facts)) ?? first;
    return combineCore(outcome, await runCoreSide(use, occurrence, signal), facts);
  }
  const stop = first.event === 'Stop';
  const [outcome, side] = await Promise.all([claude(!stop), runCoreSide(use, first, signal)]);
  const combined = combineCore(outcome, side, facts);
  if (!stop) return combined;
  const where = { sessionId: use.session.sessionId, facts };
  return request.stopCap.apply(combined, where, use.ports.environment);
}
