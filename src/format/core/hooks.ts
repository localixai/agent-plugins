import { isRecord } from '../../protocol/is-record.ts';
import { error, type PluginIssue } from '../issues.ts';
import { commandProblem } from './command.ts';
import { documentMembers } from './document.ts';
import { extensionsProblem, implementedExtensions, type CoreExtensions } from './extensions.ts';
import type { CoreVersion } from './versions.ts';

/**
 * A core package's `hooks.json` (core §7.3.1, §7.3.2): the outer object disables the portable hooks
 * component when it fails (item 2); a member whose event the core does not define, or whose value is
 * not a nonempty array, is skipped (item 3); a handler that is not a closed `{ command, args?,
 * extensions? }` with a valid command token is skipped (item 4). Each skip is reported; the rest
 * loads. The schema is `schemas/1.1.0/hooks.schema.json` (`hooks`, `#/$defs/handlers`, `handler`).
 */

/** The five events of the core (core §7.3.4–§7.3.8, §11.1 item 6). */
export const CORE_HOOK_EVENTS = [
  'SessionStart',
  'UserPromptSubmit',
  'PreToolUse',
  'PostToolUse',
  'Stop',
] as const;
export type CoreHookEvent = (typeof CORE_HOOK_EVENTS)[number];

export interface CoreHookHandler {
  readonly command: string;
  readonly args?: readonly string[];
  readonly extensions?: CoreExtensions;
}

export type CoreHooks = { readonly [E in CoreHookEvent]?: readonly CoreHookHandler[] };

/** Where each kept handler sits in its event's array of the file: `positions.<Event>[i]` is the index of `hooks.<Event>[i]`. */
export type CoreHookPositions = { readonly [E in CoreHookEvent]?: readonly number[] };

export interface CoreHooksResult {
  readonly hooks: CoreHooks;
  /** The file index of each kept handler, so a later issue names the handler it means. */
  readonly positions: CoreHookPositions;
  /** The portable hooks component is disabled: the outer object failed. */
  readonly disabled: boolean;
  readonly issues: readonly PluginIssue[];
}

const FILE = 'hooks.json';

const isEvent = (name: string): name is CoreHookEvent =>
  CORE_HOOK_EVENTS.some((event) => event === name);

const argsProblem = (value: unknown): string | undefined =>
  Array.isArray(value) && value.every((one) => typeof one === 'string')
    ? undefined
    : 'must be an array of strings';

const HANDLER_FIELDS: Readonly<Record<string, (value: unknown) => string | undefined>> = {
  command: commandProblem,
  args: argsProblem,
  extensions: extensionsProblem,
};

function handlerProblem(raw: unknown): string | undefined {
  if (!isRecord(raw)) return 'a handler must be an object';
  if (raw.command === undefined) return '"command" is required';
  for (const [field, value] of Object.entries(raw)) {
    const check = HANDLER_FIELDS[field];
    if (check === undefined) return `unknown field "${field}"`;
    const problem = check(value);
    if (problem !== undefined) return `"${field}" ${problem}`;
  }
  return undefined;
}

function handlerOf(
  raw: Readonly<Record<string, unknown>>,
  implemented: readonly string[],
): CoreHookHandler {
  const extensions = implementedExtensions(raw.extensions, implemented);
  // `handlerProblem` passed: `command` is a string and `args`, when present, strings.
  const handler: CoreHookHandler = {
    command: String(raw.command),
    ...(Array.isArray(raw.args) && { args: raw.args.map(String) }),
  };
  return extensions === undefined ? handler : { ...handler, extensions };
}

/** The handlers of one event member, each invalid one reported and skipped. */
function eventHandlers(
  event: CoreHookEvent,
  entries: readonly unknown[],
  implemented: readonly string[],
): {
  readonly handlers: CoreHookHandler[];
  readonly positions: number[];
  readonly issues: PluginIssue[];
} {
  const handlers: CoreHookHandler[] = [];
  const positions: number[] = [];
  const issues: PluginIssue[] = [];
  entries.forEach((entry, index) => {
    const problem = handlerProblem(entry);
    if (problem === undefined && isRecord(entry)) {
      handlers.push(handlerOf(entry, implemented));
      positions.push(index);
    } else issues.push(error(`hooks.${event}[${index}]`, `Handler is skipped: ${problem ?? ''}`));
  });
  return { handlers, positions, issues };
}

function memberIssue(name: string, value: unknown): PluginIssue | undefined {
  if (!isEvent(name))
    return error(`hooks.${name}`, `Event "${name}" is skipped: the core defines no such event`);
  return Array.isArray(value) && value.length > 0
    ? undefined
    : error(
        `hooks.${name}`,
        `Event "${name}" is skipped: its value must be a nonempty array of handlers`,
      );
}

/** Reads a parsed `hooks.json` against the version `plugin.json` targets. */
export function readCoreHooks(
  raw: unknown,
  manifestVersion: CoreVersion,
  implemented: readonly string[] = [],
): CoreHooksResult {
  const outer = documentMembers(raw, 'hooks', 'hooks', manifestVersion);
  if ('problem' in outer) {
    return {
      hooks: {},
      positions: {},
      disabled: true,
      issues: [error(FILE, `Hooks are disabled: ${outer.problem}`)],
    };
  }
  const hooks: { [E in CoreHookEvent]?: readonly CoreHookHandler[] } = {};
  const positions: { [E in CoreHookEvent]?: readonly number[] } = {};
  const issues: PluginIssue[] = [];
  for (const [name, value] of Object.entries(outer.members)) {
    const skipped = memberIssue(name, value);
    if (skipped !== undefined || !isEvent(name) || !Array.isArray(value)) {
      if (skipped) issues.push(skipped);
      continue;
    }
    const read = eventHandlers(name, value, implemented);
    issues.push(...read.issues);
    if (read.handlers.length > 0) {
      hooks[name] = read.handlers;
      positions[name] = read.positions;
    }
  }
  return { hooks, positions, disabled: false, issues };
}
