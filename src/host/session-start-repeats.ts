import type { BackgroundDelivery, HookOutcome } from '../hooks/index.ts';
import type { HookEvent } from '../protocol/official.ts';

/**
 * The repeat check of async `SessionStart` hooks (hooks.md, "Limitations"; Agent Plugins Extended
 * §7.3.3 item 7): the hook runs every time the event fires, and its output is skipped when its
 * `additionalContext` and `systemMessage` equal what Claude last received from that hook, unless a
 * synchronous `SessionStart` hook added text since. A compaction or `/clear` forgets what each hook
 * gave, so the first run after it delivers again.
 *
 * Judgment (spec/judgments.md): "what Claude last received" is the last delivery of that handler
 * the session handed on, and any text a synchronous `SessionStart` hook adds counts as "other text".
 */
export interface SessionStartRepeats {
  /** The background promises of one dispatch, with the repeats of a `SessionStart`'s emptied. */
  watch(outcome: HookOutcome<HookEvent>, source: unknown): readonly Promise<BackgroundDelivery>[];
}

/** The `SessionStart` sources after which every hook delivers again. */
const FORGETS: ReadonlySet<unknown> = new Set(['compact', 'clear']);

interface Given {
  readonly text: string;
  /** The count of synchronous additions when it was given. */
  readonly added: number;
}

const textOf = (delivery: BackgroundDelivery): string =>
  JSON.stringify([delivery.additionalContext ?? '', delivery.systemMessage ?? '']);

const withoutText = ({
  additionalContext: _context,
  systemMessage: _message,
  ...rest
}: BackgroundDelivery): BackgroundDelivery => rest;

export function createSessionStartRepeats(): SessionStartRepeats {
  const given = new Map<string, Given>();
  let added = 0;
  const pass = (delivery: BackgroundDelivery): BackgroundDelivery => {
    if (delivery.additionalContext === undefined && delivery.systemMessage === undefined) {
      return delivery;
    }
    const text = textOf(delivery);
    const last = given.get(delivery.handlerId);
    if (last !== undefined && last.text === text && last.added === added) {
      return withoutText(delivery);
    }
    given.set(delivery.handlerId, { text, added });
    return delivery;
  };
  return {
    watch(outcome, source) {
      if (outcome.event !== 'SessionStart') return outcome.background;
      if (FORGETS.has(source)) given.clear();
      if (outcome.additionalContext.length + outcome.systemMessages.length > 0) added += 1;
      return outcome.background.map((promise) => promise.then(pass));
    },
  };
}
