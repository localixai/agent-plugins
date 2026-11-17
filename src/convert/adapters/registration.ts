import {
  CORE_HOOK_EVENTS,
  type CoreHookEvent,
  type CoreHookHandler,
} from '../../format/core/hooks.ts';
import type { HookHandler, HookHandlerOf } from '../../protocol/hook-config.ts';
import { isProfileName, type ProfileName } from '../profiles.ts';
import { ADAPTER_RUNTIME } from './sources.ts';

/**
 * How each target's hooks file names a hook adapter (Agent Plugins Extended §18.4.4), and how a
 * converter tells its own adapter apart when it converts back: an adapter registration of exactly
 * this shape is unwrapped to the handler it runs (judgment, spec/judgments.md), so a converted
 * package converts back to the hooks it had instead of an adapter around an adapter.
 */

export const RUN_CORE_HANDLER = 'hooks/run-core-handler.mjs';
export const RUN_CLAUDE_CODE_HANDLER = 'hooks/run-claude-code-handler.mjs';

const CORE_ADAPTER_ARG = `\${CLAUDE_PLUGIN_ROOT}/${RUN_CORE_HANDLER}`;
const CLAUDE_CODE_ADAPTER_ARG = `\${PLUGIN_ROOT}/${RUN_CLAUDE_CODE_HANDLER}`;
const END = '--';

const isCoreEvent = (value: string | undefined): value is CoreHookEvent =>
  CORE_HOOK_EVENTS.some((event) => event === value);

/** A core handler's program: its command and arguments, as the form that runs it writes them. */
export interface HandlerProgram {
  readonly command: string;
  readonly args: readonly string[];
}

/** The Claude Code hook that runs a core handler; `args` already use Claude Code's placeholders. */
export const coreAdapterHook = (
  event: CoreHookEvent,
  program: HandlerProgram,
): HookHandlerOf<'command'> => ({
  type: 'command',
  command: ADAPTER_RUNTIME,
  args: [CORE_ADAPTER_ARG, event, END, program.command, ...program.args],
});

const ONLY_ADAPTER_FIELDS: ReadonlySet<string> = new Set(['type', 'command', 'args']);

/** The core handler a Claude Code hook of this converter runs, or `undefined` for any other hook. */
export function unwrapCoreAdapter(
  hook: HookHandler,
): { readonly event: CoreHookEvent; readonly program: HandlerProgram } | undefined {
  if (!Object.keys(hook).every((key) => ONLY_ADAPTER_FIELDS.has(key))) return undefined;
  if (!('args' in hook) || hook.command !== ADAPTER_RUNTIME || hook.args === undefined)
    return undefined;
  const [adapter, event, end, command, ...args] = hook.args;
  if (adapter !== CORE_ADAPTER_ARG || !isCoreEvent(event) || end !== END) return undefined;
  return command === undefined ? undefined : { event, program: { command, args } };
}

/** What a Claude Code `command` hook is made of, as the adapter that runs it under the core takes it. */
export interface ClaudeCodeRun {
  readonly event: CoreHookEvent;
  /** Run only on calls of this profile: the matcher selected its tools. */
  readonly profile?: ProfileName;
  readonly timeout?: number;
  readonly shell?: 'bash' | 'powershell';
  /** Exec form: the hook had `args`. */
  readonly exec: boolean;
  readonly command: string;
  readonly args: readonly string[];
}

const flag = (name: string, value: string | number | undefined): string[] =>
  value === undefined ? [] : [name, String(value)];

/** The core handler that runs a Claude Code hook through the adapter. */
export const claudeCodeAdapterHandler = (run: ClaudeCodeRun): CoreHookHandler => ({
  command: ADAPTER_RUNTIME,
  args: [
    CLAUDE_CODE_ADAPTER_ARG,
    ...flag('--event', run.event),
    ...flag('--profile', run.profile),
    ...flag('--timeout', run.timeout),
    ...flag('--shell', run.shell),
    ...(run.exec ? ['--exec'] : []),
    END,
    run.command,
    ...run.args,
  ],
});

const SHELLS = ['bash', 'powershell'] as const;
type Shell = (typeof SHELLS)[number];
const isShell = (value: string): value is Shell => SHELLS.some((shell) => shell === value);

const VALUED = ['--event', '--profile', '--timeout', '--shell'];

/** The flags of an adapter handler, by name; `undefined` for a flag list this converter never writes. */
function flagsOf(flags: readonly string[]): ReadonlyMap<string, string> | undefined {
  const found = new Map<string, string>();
  for (let at = 0; at < flags.length; at += 1) {
    const name = flags[at] ?? '';
    if (name === '--exec') found.set(name, '');
    else if (VALUED.includes(name) && at + 1 < flags.length) {
      found.set(name, flags[at + 1] ?? '');
      at += 1;
    } else return undefined;
  }
  return found;
}

type Options = Pick<ClaudeCodeRun, 'profile' | 'timeout' | 'shell'>;

/** Each optional flag read back, or `undefined` when its value is not one this converter writes. */
const READERS: { readonly [flag: string]: (value: string) => Options | undefined } = {
  '--profile': (value) => (isProfileName(value) ? { profile: value } : undefined),
  '--timeout': (value) => (Number(value) > 0 ? { timeout: Number(value) } : undefined),
  '--shell': (value) => (isShell(value) ? { shell: value } : undefined),
};

function optionsOf(flags: ReadonlyMap<string, string>): Options | undefined {
  let options: Options = {};
  for (const [name, read] of Object.entries(READERS)) {
    const value = flags.get(name);
    if (value === undefined) continue;
    const one = read(value);
    if (one === undefined) return undefined;
    options = { ...options, ...one };
  }
  return options;
}

/** The flags and the hook's command and arguments of an adapter handler's `args`. */
function partsOf(args: readonly string[]) {
  const end = args.indexOf(END);
  if (end < 0 || args[0] !== CLAUDE_CODE_ADAPTER_ARG) return undefined;
  const flags = flagsOf(args.slice(1, end));
  const command = args[end + 1];
  return flags === undefined || command === undefined
    ? undefined
    : { flags, command, args: args.slice(end + 2) };
}

/** The Claude Code hook a core handler of this converter runs, or `undefined` for any other handler. */
export function unwrapClaudeCodeAdapter(handler: CoreHookHandler): ClaudeCodeRun | undefined {
  const parts = handler.command === ADAPTER_RUNTIME ? partsOf(handler.args ?? []) : undefined;
  if (parts === undefined) return undefined;
  const event = parts.flags.get('--event');
  const options = optionsOf(parts.flags);
  if (options === undefined || !isCoreEvent(event)) return undefined;
  return {
    event,
    ...options,
    exec: parts.flags.has('--exec'),
    command: parts.command,
    args: parts.args,
  };
}
