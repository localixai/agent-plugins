import {
  CORE_HOOK_EVENTS,
  type CoreHookEvent,
  type CoreHookHandler,
  type CoreHooks,
} from '../../format/core/hooks.ts';
import type { HookMatcherGroup, HooksSettings } from '../../protocol/hook-config.ts';
import {
  coreAdapterHook,
  unwrapClaudeCodeAdapter,
  type ClaudeCodeRun,
} from '../adapters/registration.ts';
import { notConverted } from '../notes.ts';
import { holdsOtherReference, toClaudeCodePlaceholders } from '../placeholders.ts';
import { matcherOf } from '../profiles.ts';
import type { ConversionNote } from '../types.ts';

/**
 * `hooks/hooks.json` from a core `hooks.json` (Agent Plugins Extended §18.4.2 item 4, J18.17): every
 * core handler runs through the hook adapter, one exec-form `command` hook each, registered as
 * §18.3.4 item 3 says: a `SessionStart` handler with the matcher `startup|clear|fork`, a
 * `PostToolUse` handler on `PostToolUse` and on `PostToolUseFailure`, the others on the event of the
 * same name, with no matcher. `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` in `args` become Claude Code's
 * names; a handler with any other `${` in its `args`, which Claude Code would substitute (§18.4.5), is
 * not converted. A handler this converter wrote around a Claude Code hook is unwrapped to that hook.
 * A `UserPromptSubmit` handler is listed as also running on the turns the adapter cannot tell from an
 * external submission (§18.4.4 item 5, J18.23).
 */

export const CLAUDE_HOOKS = 'hooks/hooks.json';
const SOURCE = 'hooks.json';
const NEW_SESSIONS = 'startup|clear|fork';

/** Where each core event's handler is registered in Claude Code, with the matcher each needs. */
const REGISTRATIONS: {
  readonly [E in CoreHookEvent]: readonly { readonly event: string; readonly matcher?: string }[];
} = {
  SessionStart: [{ event: 'SessionStart', matcher: NEW_SESSIONS }],
  UserPromptSubmit: [{ event: 'UserPromptSubmit' }],
  PreToolUse: [{ event: 'PreToolUse' }],
  PostToolUse: [{ event: 'PostToolUse' }, { event: 'PostToolUseFailure' }],
  Stop: [{ event: 'Stop' }],
};

/** The matcher a hook adapter of this converter is registered with on a core event's own event. */
export const adapterMatcher = (event: CoreHookEvent): string | undefined =>
  REGISTRATIONS[event][0]?.matcher;

interface Placed {
  readonly event: string;
  readonly group: HookMatcherGroup;
}

const group = (hook: HookMatcherGroup['hooks'][number], matcher?: string): HookMatcherGroup => ({
  ...(matcher === undefined ? {} : { matcher }),
  hooks: [hook],
});

/** A Claude Code hook the adapter ran under the core, back in its own place. */
function unwrapped(run: ClaudeCodeRun): Placed {
  const hook = {
    type: 'command' as const,
    command: run.command,
    ...(run.exec ? { args: [...run.args] } : {}),
    ...(run.timeout === undefined ? {} : { timeout: run.timeout }),
    ...(run.shell === undefined ? {} : { shell: run.shell }),
  };
  const matcher = run.profile === undefined ? adapterMatcher(run.event) : matcherOf(run.profile);
  return { event: run.event, group: group(hook, matcher) };
}

interface HandlerPart {
  readonly placed: readonly Placed[];
  readonly adapted: boolean;
  readonly notes: readonly ConversionNote[];
}

function handlerPart(
  event: CoreHookEvent,
  handler: CoreHookHandler,
  field: string,
  adapter: boolean,
): HandlerPart {
  const run = unwrapClaudeCodeAdapter(handler);
  if (run !== undefined && run.event === event) {
    return { placed: [unwrapped(run)], adapted: false, notes: [] };
  }
  const args = handler.args ?? [];
  const label = [handler.command, ...args].join(' ');
  if (!adapter) {
    const message = `handler "${label}" not converted: a core handler needs a hook adapter (§18.4.4)`;
    return { placed: [], adapted: false, notes: [notConverted(SOURCE, field, message)] };
  }
  if (args.some((arg) => holdsOtherReference(arg, 'core'))) {
    const message = `handler "${label}" not converted: its args hold "\${" text, which Claude Code would substitute`;
    return { placed: [], adapted: false, notes: [notConverted(SOURCE, field, message)] };
  }
  const hook = coreAdapterHook(event, {
    command: handler.command,
    args: args.map(toClaudeCodePlaceholders),
  });
  const placed = REGISTRATIONS[event].map((at) => ({
    event: at.event,
    group: group(hook, at.matcher),
  }));
  return { placed, adapted: true, notes: OVER_FIRING[event]?.(field, label) ?? [] };
}

/** What an adapted handler does under Claude Code that its core event does not (J18.23). */
const OVER_FIRING: Partial<
  Readonly<Record<CoreHookEvent, (field: string, label: string) => ConversionNote[]>>
> = {
  UserPromptSubmit: (field, label) => [
    notConverted(
      SOURCE,
      field,
      `handler "${label}" also runs on task notifications and automatic continuations: Claude Code gives them the source "system" of external turns (§18.4.4 item 5)`,
    ),
  ],
};

const extensionNotes = (handler: CoreHookHandler, field: string): ConversionNote[] =>
  Object.keys(handler.extensions ?? {}).map((namespace) =>
    notConverted(
      SOURCE,
      `${field}.extensions.${namespace}`,
      'client extension data has no Claude Code counterpart',
    ),
  );

export interface HooksPart {
  readonly hooks: HooksSettings;
  /** At least one handler runs through the adapter, which must then be written. */
  readonly adapted: boolean;
  readonly notes: readonly ConversionNote[];
}

export function claudeCodeHooks(hooks: CoreHooks, adapter: boolean): HooksPart {
  const parts = CORE_HOOK_EVENTS.flatMap((event) =>
    (hooks[event] ?? []).map((handler, index) => {
      const field = `hooks.${event}[${index}]`;
      const part = handlerPart(event, handler, field, adapter);
      return { ...part, notes: [...part.notes, ...extensionNotes(handler, field)] };
    }),
  );
  const byEvent = new Map<string, HookMatcherGroup[]>();
  for (const { event, group: placed } of parts.flatMap((part) => part.placed)) {
    byEvent.set(event, [...(byEvent.get(event) ?? []), placed]);
  }
  return {
    hooks: Object.fromEntries(byEvent),
    adapted: parts.some((part) => part.adapted),
    notes: parts.flatMap((part) => part.notes),
  };
}
