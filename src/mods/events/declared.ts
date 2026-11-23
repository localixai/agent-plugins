import { z } from 'zod';
import type { DECLARED_EVENT_NAMES } from '../../protocol/mods-events.ts';
import { answer, count, fields, flag, passThrough, text, type EventSpec } from './spec.ts';

/**
 * The engine's events only the 2.1.330 declarations define (spec Judgment J10.22; §10.16, §10.19.3,
 * §10.19.4). `prompt.change` has its own file.
 */
type DeclaredEvent = Exclude<(typeof DECLARED_EVENT_NAMES)[number], 'prompt.change'>;

/** `{ plugin, tier }` of whoever defined the tool or the agent. */
const provider = z.looseObject({ plugin: text, tier: text });

export const DECLARED_SPECS: { readonly [N in DeclaredEvent]: EventSpec } = {
  // `ToolOfferInput` (claude-code/index.d.ts:5400): `{ isOffered: false }` keeps the tool from the
  // loop's model; `next(e)` resolves `{ isOffered: true }`; a hook that fails passes the tool through.
  'tool.offer': {
    input: fields({
      tool: text,
      provider,
      agentId: text.optional(),
      subagentType: text.optional(),
    }),
    result: answer({ isOffered: flag }),
  },
  // `AgentReportInput` (:5219): what the outermost hook returns is delivered, `{ text }` or
  // `{ consumed: reason }`.
  'agent.report': {
    input: fields({
      agentId: text,
      parentAgentId: text.optional(),
      subagentType: text,
      status: z.enum(['completed', 'failed', 'stopped']),
      text,
      provider,
    }),
    result: answer({ text: text.optional(), consumed: text.optional() }),
  },
  // `UiPointerInput` (:5134): a click on the backdrop, seen only by the mod that set the theme;
  // observe only.
  'ui.pointer': {
    input: fields({
      type: z.literal('click'),
      x: count,
      y: count,
      scene: text,
      component: z.literal('Backdrop'),
    }),
    result: passThrough,
  },
};
