import type { HookEvent } from '../../protocol/official.ts';
import { factOf, textFactOf } from '../input/fact-read.ts';
import { completedProfile, type Completed } from './profile-results.ts';
import { pendingProfile } from './profiles.ts';
import type { CoreCallFacts, CoreOccurrence, CoreToolProfile } from './types.ts';

/**
 * Which Claude Code occurrences are also core occurrences, and the core input each gives (Agent
 * Plugins Extended §18.1.2, the table and "Inputs"; §18.2): exactly the core fields (core
 * §7.3.3–§7.3.8), the common ones with the values Claude Code's handlers get, never a Claude Code
 * field. An event without a core counterpart, and an occurrence the table excludes, gives none.
 */

export interface OccurrenceContext {
  readonly sessionId: string;
  readonly cwd: string;
  /** The hooks fire in a subagent: core `SessionStart` does not run for one (core §7.3.4). */
  readonly subagent: boolean;
  readonly facts: CoreCallFacts;
}

type Facts = object;
type Builder = (facts: Facts, context: OccurrenceContext) => CoreOccurrence | undefined;

/** J18.6: a new root session with a new `session_id`; `resume` and `compact` keep the id. */
const NEW_SESSION: ReadonlySet<string> = new Set(['startup', 'clear', 'fork']);

/**
 * J18.7: the `source` values of the turns Claude Code starts itself (a scheduled task, a `/loop`
 * wakeup). `system` holds both kinds ("peer/channel messages, task notifications,
 * auto-continuation"), so only the host's `CoreCallFacts.turnOrigin` decides it, never a default
 * (`needsTurnOrigin`); an absent `source` is a submitted prompt unless the host says otherwise.
 */
const STARTED_BY_CLIENT: ReadonlySet<string> = new Set(['loop_wakeup', 'schedule_wakeup']);
const BY_ORIGIN_ONLY: ReadonlySet<string> = new Set(['system']);
const ORIGIN_EVENTS: ReadonlySet<HookEvent> = new Set<HookEvent>(['UserPromptSubmit']);
const FIRES: { readonly [O in NonNullable<CoreCallFacts['turnOrigin']>]: boolean } = {
  external: true,
  client: false,
};

const sourceOf = (facts: Facts): string => textFactOf(facts, 'source') ?? '';

/** Whether a `UserPromptSubmit` turn is one only the host can classify and the host did not. */
export const needsTurnOrigin = (event: HookEvent, facts: Facts, core: CoreCallFacts): boolean =>
  ORIGIN_EVENTS.has(event) && core.turnOrigin === undefined && BY_ORIGIN_ONLY.has(sourceOf(facts));

/** Whether a `UserPromptSubmit` turn is an external submission (J18.7). */
function isExternal(facts: Facts, core: CoreCallFacts): boolean {
  if (core.turnOrigin !== undefined) return FIRES[core.turnOrigin];
  const source = sourceOf(facts);
  return !BY_ORIGIN_ONLY.has(source) && !STARTED_BY_CLIENT.has(source);
}

const common = (event: CoreOccurrence['event'], context: OccurrenceContext) => ({
  hook_event_name: event,
  session_id: context.sessionId,
  cwd: context.cwd,
});

const withProfile = (input: Record<string, unknown>, profile: CoreToolProfile | undefined) =>
  profile === undefined ? input : { ...input, tool_profile: profile };

const call = (facts: Facts) => ({
  toolName: textFactOf(facts, 'tool_name') ?? '',
  toolUseId: textFactOf(facts, 'tool_use_id') ?? '',
  toolInput: factOf(facts, 'tool_input') ?? {},
});

function toolOccurrence(
  event: 'PreToolUse' | 'PostToolUse',
  facts: Facts,
  context: OccurrenceContext,
  after?: { readonly completed: Completed; readonly result: unknown },
): CoreOccurrence {
  const { toolName, toolUseId, toolInput } = call(facts);
  const base = {
    ...common(event, context),
    tool_use_id: toolUseId,
    tool_name: toolName,
    tool_input: toolInput,
  };
  const profileContext = { cwd: context.cwd, facts: context.facts };
  const input =
    after === undefined
      ? withProfile(base, pendingProfile(toolName, toolInput, profileContext))
      : withProfile(
          { ...base, tool_result: after.result },
          completedProfile(toolName, toolInput, after.completed, profileContext),
        );
  return { event, input, toolName };
}

const BUILDERS: { readonly [E in HookEvent]?: Builder } = {
  SessionStart: (facts, context) =>
    context.subagent || !NEW_SESSION.has(textFactOf(facts, 'source') ?? '')
      ? undefined
      : { event: 'SessionStart', input: common('SessionStart', context) },
  UserPromptSubmit: (facts, context) => {
    if (!isExternal(facts, context.facts)) return undefined;
    const prompt = textFactOf(facts, 'prompt') ?? '';
    return {
      event: 'UserPromptSubmit',
      input: { ...common('UserPromptSubmit', context), prompt },
    };
  },
  // `EndConversation` fires no tool event in Claude Code either; the host does not dispatch one.
  PreToolUse: (facts, context) => toolOccurrence('PreToolUse', facts, context),
  PostToolUse: (facts, context) => {
    const response = factOf(facts, 'tool_response');
    // J18.2: `tool_result` is the tool's own result, as Claude Code's `tool_response` carries it;
    // an MCP call's is the server's `CallToolResult` when the host gives it.
    const result = context.facts.mcpResult ?? response;
    return toolOccurrence('PostToolUse', facts, context, {
      completed: { ok: true, response },
      result,
    });
  },
  // J18.8: a failure the tool reports is a completed result; an interrupt is not.
  PostToolUseFailure: (facts, context) => {
    if (factOf(facts, 'is_interrupt') === true) return undefined;
    const error = textFactOf(facts, 'error') ?? '';
    return toolOccurrence('PostToolUse', facts, context, {
      completed: { ok: false, error },
      result: context.facts.mcpResult ?? error,
    });
  },
  // J18.9: one continuation mechanism, so `continuation_active` is `stop_hook_active`.
  Stop: (facts, context) => ({
    event: 'Stop',
    input: {
      ...common('Stop', context),
      continuation_active: factOf(facts, 'stop_hook_active') === true,
    },
  }),
};

/** The core occurrence of a Claude Code dispatch, or `undefined` when the core event does not fire. */
export function coreOccurrence(
  event: HookEvent,
  facts: Facts,
  context: OccurrenceContext,
): CoreOccurrence | undefined {
  return BUILDERS[event]?.(facts, context);
}
