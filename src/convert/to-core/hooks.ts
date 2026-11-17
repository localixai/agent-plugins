import {
  CORE_HOOK_EVENTS,
  type CoreHookEvent,
  type CoreHookHandler,
} from '../../format/core/hooks.ts';
import { defined } from '../../format/defined.ts';
import type { HookHandler, HookHandlerOf, HooksSettings } from '../../protocol/hook-config.ts';
import { claudeCodeAdapterHandler, unwrapCoreAdapter } from '../adapters/registration.ts';
import { adapterMatcher } from '../to-claude-code/hooks.ts';
import { notConverted, reported } from '../notes.ts';
import { holdsCorePlaceholder, toCorePlaceholders } from '../placeholders.ts';
import type { ConversionNote } from '../types.ts';
import { readMatcher } from './hook-matchers.ts';

/**
 * A core `hooks.json` from a Claude Code plugin's hooks (Agent Plugins Extended §18.4.3 item 4,
 * J18.17): only `command` hooks on the five shared events convert, each through the hook adapter,
 * limited to the profile its matcher selects. Hooks on the other events, the other handler types,
 * `if`, `async`, `asyncRewake`, `once` and `onFailure` are not converted; `statusMessage` is dropped
 * and reported. A hook naming `${user_config.*}`, or the core's own `${PLUGIN_ROOT}` and
 * `${PLUGIN_DATA}` (which a core client would expand in the adapter's arguments), is not converted.
 * A Claude Code hook this converter wrote around a core handler is unwrapped to that handler, and its
 * `PostToolUseFailure` twin is dropped quietly (judgment, spec/judgments.md). Positions are those of
 * the plugin's merged hooks.
 */

export const CORE_HOOKS = 'hooks.json';
const SOURCE = 'hooks/hooks.json';

type CommandHook = HookHandlerOf<'command'>;

const REFUSED_FIELDS = ['if', 'async', 'asyncRewake', 'once', 'onFailure'] as const;

const isCoreEvent = (value: string): value is CoreHookEvent =>
  CORE_HOOK_EVENTS.some((event) => event === value);

interface Converted {
  readonly handler?: CoreHookHandler;
  /** The handler runs through the adapter, which must then be written. */
  readonly adapted?: boolean;
  readonly notes: readonly ConversionNote[];
}

const refused = (field: string, message: string): Converted => ({
  notes: [notConverted(SOURCE, field, message)],
});

/** Why a command hook cannot run under the core, or `undefined`. */
function commandProblem(hook: CommandHook): string | undefined {
  const fields = REFUSED_FIELDS.filter((key) => hook[key] !== undefined);
  if (fields.length > 0)
    return `${fields.map((key) => `"${key}"`).join(', ')} has no core counterpart`;
  const texts = [hook.command, ...(hook.args ?? [])];
  if (texts.some((text) => text.includes('${user_config.'))) {
    return 'it names ${user_config.*}, which the core has no counterpart for';
  }
  return texts.some(holdsCorePlaceholder)
    ? 'it holds ${PLUGIN_ROOT} or ${PLUGIN_DATA}, which a core client would expand'
    : undefined;
}

/** A core handler this converter wrapped, back as it was: registered exactly as it registers one. */
function unwrapped(event: CoreHookEvent, hook: HookHandler, matcher: string | undefined) {
  const found = unwrapCoreAdapter(hook);
  if (found === undefined || found.event !== event || matcher !== adapterMatcher(event)) {
    return undefined;
  }
  const args = found.program.args.map(toCorePlaceholders);
  return { command: found.program.command, ...(args.length === 0 ? {} : { args }) };
}

interface HookSite {
  readonly event: CoreHookEvent;
  readonly matcher: string | undefined;
  readonly field: string;
  readonly adapter: boolean;
}

function notes(site: HookSite, hook: CommandHook, narrowed: string | undefined): ConversionNote[] {
  const { field } = site;
  const name = `hook "${hook.command}"`;
  return [
    ...(hook.statusMessage === undefined
      ? []
      : [
          reported(
            SOURCE,
            `${field}.statusMessage`,
            '"statusMessage" has no core counterpart: dropped',
          ),
        ]),
    ...(narrowed === undefined ? [] : [reported(SOURCE, field, `${name}: ${narrowed}`)]),
    ...(site.event === 'PostToolUse'
      ? [
          reported(
            SOURCE,
            field,
            `${name} also runs on failed calls, which the core's PostToolUse includes`,
          ),
        ]
      : []),
  ];
}

/** The adapter handler of a command hook that converts, with what it reports. */
function adapted(site: HookSite, hook: CommandHook): Converted {
  const reading = readMatcher(site.event, site.matcher);
  const problem =
    commandProblem(hook) ??
    (site.adapter ? undefined : 'it needs a hook adapter (§18.4.4)') ??
    ('problem' in reading ? reading.problem : undefined);
  if (problem !== undefined || 'problem' in reading) {
    return refused(site.field, `hook "${hook.command}" not converted: ${problem ?? ''}`);
  }
  const handler = claudeCodeAdapterHandler({
    event: site.event,
    ...defined({ profile: reading.profile, timeout: hook.timeout, shell: hook.shell }),
    exec: hook.args !== undefined,
    command: hook.command,
    args: hook.args ?? [],
  });
  return { handler, adapted: true, notes: notes(site, hook, reading.narrowed) };
}

function convertHook(site: HookSite, hook: HookHandler): Converted {
  const original = unwrapped(site.event, hook, site.matcher);
  if (original !== undefined) return { handler: original, notes: [] };
  if (!('command' in hook)) {
    return refused(site.field, `a "${hook.type}" hook has no core counterpart`);
  }
  return adapted(site, hook);
}

/** Whether a `PostToolUseFailure` hook is the twin of a `PostToolUse` adapter this converter wrote. */
const isAdapterTwin = (hook: HookHandler): boolean =>
  unwrapCoreAdapter(hook)?.event === 'PostToolUse';

export interface CoreHooksPart {
  readonly hooks: { readonly [E in CoreHookEvent]?: readonly CoreHookHandler[] };
  readonly adapted: boolean;
  readonly notes: readonly ConversionNote[];
}

function eventParts(event: string, groups: HooksSettings[string], adapter: boolean) {
  return groups.flatMap((group, at) =>
    group.hooks.map((hook, index) => {
      const field = `hooks.${event}[${at}].hooks[${index}]`;
      if (isCoreEvent(event)) {
        return { event, ...convertHook({ event, matcher: group.matcher, field, adapter }, hook) };
      }
      if (event === 'PostToolUseFailure' && group.matcher === undefined && isAdapterTwin(hook)) {
        return { event, notes: [] };
      }
      return { event, ...refused(field, `hooks on ${event} have no core counterpart`) };
    }),
  );
}

export function coreHooksOf(hooks: HooksSettings, adapter: boolean): CoreHooksPart {
  const parts = Object.entries(hooks).flatMap(([event, groups]) =>
    eventParts(event, groups, adapter),
  );
  const kept = CORE_HOOK_EVENTS.flatMap((event) => {
    const handlers = parts.flatMap((part) =>
      part.event === event && part.handler !== undefined ? [part.handler] : [],
    );
    return handlers.length === 0 ? [] : [[event, handlers] as const];
  });
  return {
    hooks: Object.fromEntries(kept),
    adapted: parts.some((part) => part.adapted === true),
    notes: parts.flatMap((part) => part.notes),
  };
}
