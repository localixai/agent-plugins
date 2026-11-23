import { z } from 'zod';
import { SESSION_END_REASONS } from '../../protocol/exit-reasons.ts';
import type { MOD_EVENT_GROUPS } from '../../protocol/mods-events.ts';
import {
  answer,
  count,
  fields,
  flag,
  noFields,
  passThrough,
  text,
  type EventSpec,
} from './spec.ts';
import { isRecord } from '../../protocol/is-record.ts';

/** The session events (reference.md, "Session"; api.md, "Send and receive messages between sessions"). */
type SessionEvent = (typeof MOD_EVENT_GROUPS.session)[number];

/** `{ isDelivered: false, reason }`: a message that was not delivered says why. */
const deliveryResult = answer({ isDelivered: flag.optional(), reason: text.optional() }).refine(
  (result) => result.isDelivered !== false || typeof result.reason === 'string',
  { error: 'a message that was not delivered needs a reason' },
);

/**
 * `session.append` is called once for each row the conversation keeps. A hook passes a changed
 * `message.content`, to rewrite the text blocks of the row or the `content` of a `tool_result`
 * block in it. Judgment: any other change to the event is ignored, so Claude Code stores the row
 * as fired with the last `content` the hooks passed on (the docs name no other change).
 */
function appendedRow(original: unknown, passed: unknown): unknown {
  if (!isRecord(original) || !isRecord(passed)) return passed;
  const before = original.message;
  const after = passed.message;
  if (!isRecord(before) || !isRecord(after) || !('content' in after)) return original;
  return { ...original, message: { ...before, content: after.content } };
}

export const SESSION_SPECS: { readonly [N in SessionEvent]: EventSpec } = {
  'session.start': { input: noFields, result: passThrough },
  // `e.sessionId` and `e.resume.id` are the ending session's (claude-code/index.d.ts:12106).
  'session.end': {
    input: fields({
      reason: z.enum(SESSION_END_REASONS),
      sessionId: text.optional(),
      resume: z.looseObject({ id: text }).optional(),
    }),
    result: passThrough,
  },
  // A hook answers `{ skip: reason }` to stop the compaction. Each message may carry its `uuid`
  // (2.1.330, spec §10.19.2).
  'session.compact': {
    input: fields({ messages: z.array(z.looseObject({ uuid: text.optional() })).optional() }),
    result: answer({ skip: text.optional() }),
  },
  // `e.text` is the message and `e.origin.kind` says where it came from (`peer`,
  // `peer-send-message`, `task-notification`, `scheduled-trigger`...); `{ consumed: reason }` keeps it from Claude.
  'session.receive': {
    input: fields({ text, origin: z.looseObject({ kind: text }) }),
    result: answer({ consumed: text.optional(), text: text.optional() }),
  },
  // `e.to` arrives as a string, `e.origin.kind` is `model` or `plugin`.
  'session.send': {
    input: fields({
      to: text,
      text,
      origin: z.looseObject({ kind: z.enum(['model', 'plugin']) }),
    }),
    result: deliveryResult,
  },
  // `e.door` is the kind of stored row.
  'session.append': {
    input: fields({
      message: z.looseObject({ content: z.unknown() }),
      door: text.optional(),
    }),
    result: passThrough,
    coreInput: appendedRow,
  },
  'session.attach': { input: noFields, result: passThrough },
  'session.detach': { input: noFields, result: passThrough },
  // `cost` is `{ usd, hasUnknownPrice? }` (claude-code/index.d.ts:14657 of 2.1.330).
  'session.measure': {
    input: fields({
      cost: z.looseObject({ usd: count, hasUnknownPrice: flag.optional() }).optional(),
    }),
    result: passThrough,
  },
  // §10.19.2: observe only; the stop goes on whatever a hook does, and `next(e)` resolves
  // `{ isTurnRunning }` (`SessionStopInput`, claude-code/index.d.ts:5900).
  'session.stop': { input: fields({ isTurnRunning: flag }), result: passThrough },
};
