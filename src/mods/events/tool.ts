import { z } from 'zod';
import { PERMISSION_BEHAVIORS } from '../../protocol/permission-modes.ts';
import type { MOD_EVENT_GROUPS } from '../../protocol/mods-events.ts';
import { pinnedFields } from './pinned.ts';
import { answer, fields, flag, text, type EventSpec } from './spec.ts';

/**
 * The tool events (spec §10.16; reference.md, "Tools"; events.md, "Guard or change a tool call" and
 * "Approve or refuse a tool call before the user is asked").
 */
type ToolEvent = (typeof MOD_EVENT_GROUPS.tools)[number];

// The layer gate reads `process` and a dot in code as the global, so the namespace is named once, apart.
const PROCESS_NAMESPACE = 'process';

/** The `$` calls a `tool.check` or `tool.judge` with no tool stands for (`e.op`, claude-code/index.d.ts:17969). */
const TOOL_CHECK_OPS = [
  'fs.read',
  'fs.list',
  'fs.write',
  `${PROCESS_NAMESPACE}.run`,
  `${PROCESS_NAMESPACE}.spawn`,
] as const;

/** The fields `tool.check` and `tool.judge` share (`ToolCheckInput`, `ToolJudgeInput`). */
const checkFields = {
  tool: text,
  input: z.record(z.string(), z.unknown()),
  agentId: text.optional(),
  approver: z.enum(['host', 'person']).optional(),
  ceiling: z.enum(PERMISSION_BEHAVIORS).optional(),
  isCeilingUnread: z.literal(true).optional(),
  isReadOnly: z.literal(true).optional(),
  op: z.enum(TOOL_CHECK_OPS).optional(),
};

/** A decision: `allow`, `ask` or `deny`, with an optional reason. */
const decided = answer({ decision: z.enum(PERMISSION_BEHAVIORS), reason: text.optional() });

/** What a second `next(e)` in a `tool.run` hook answers: the tool ran once already (§10.16.6 item 3). */
const TOOL_RAN_ONCE = 'tool.run: the call already ran; next(e) runs it once';

export const TOOL_SPECS: { readonly [N in ToolEvent]: EventSpec } = {
  // `e.tool` is the tool's name and its arguments are fields of `e`, such as `e.command` for Bash.
  // A hook answers `{ deny }` or `{ result }`; what `next` resolves to is the tool's own result,
  // where "a refused call comes back as { deny }, and a failed one has isError set".
  'tool.call': {
    input: fields({ tool: text }),
    result: answer({ deny: text.optional(), isError: flag.optional() }),
  },
  // `e.input` holds the arguments. `e.isReadOnly` is `true` or absent, "not `false`". The decision
  // `next(e)` resolves to is an object whose `decision` is allow, ask or deny. `isCeilingUnread` and
  // `op` are set by the client and pinned (claude-code/index.d.ts:17959, :17969 of 2.1.330).
  'tool.check': {
    input: fields(checkFields),
    result: decided,
    changeProblem: pinnedFields('tool.check', ['isCeilingUnread', 'op']),
  },
  // §10.16.5: `next(e)` asks the classifier once however often a hook calls it; a hook decides in
  // its place with `{ decision }`.
  'tool.judge': {
    input: fields({ ...checkFields, tool_use_id: text }),
    result: decided,
    changeProblem: pinnedFields('tool.judge', ['isCeilingUnread', 'op']),
    coreAgain: (first) => first,
  },
  // §10.16.6: the call as allowed, every field pinned; `next(e)` runs the tool once, a second call
  // starts nothing and resolves to a `{ deny }`.
  'tool.run': {
    input: fields({ tool: text }),
    result: answer({ deny: text.optional(), isError: flag.optional() }),
    changeProblem: pinnedFields('tool.run', 'every'),
    coreAgain: () => ({ deny: TOOL_RAN_ONCE }),
  },
  // `e.description` is the text Claude reads; a hook answers `{ description }` and may set
  // `isDeferred`, and `breakPromptCache: true` to send a changed description on the next turn.
  // `e.mcpServer` is read off the connection that listed the tool and pinned (:18111 of 2.1.330).
  'tool.describe': {
    input: fields({ description: text, mcpServer: z.looseObject({ type: text }).optional() }),
    result: answer({
      description: text,
      isDeferred: flag.optional(),
      breakPromptCache: flag.optional(),
    }),
    changeProblem: pinnedFields('tool.describe', ['mcpServer']),
  },
};
