import { ENGINE_ORIGIN } from '../../protocol/mods-events.ts';
import type { EventRuleBook, HookReport, ModChain, RegisteredHook } from '../types.ts';
import { isRecord } from '../../protocol/is-record.ts';
import { leftOutOf, providesProblem, withProvides, type PluginProvides } from './provides.ts';
import type { PluginRegisterInput } from './register-event.ts';

/**
 * `plugin.register` (Agent Plugins Extended §10.19.5, §14.9.8; plugins/mods/reference.md, "Other
 * mods"; admin.md, "Enforce a policy with a mod of your own"): fired each time a plugin is about to
 * load, with or without a hooks module (2.1.326). A hook answers `{ refuse: reason }`, which keeps
 * the whole plugin out, or `next(e)`, with entries removed from `e.provides` to keep those parts out;
 * a refusal returned after `next` resolved still holds (events.md).
 */

const REGISTER_EVENT = 'plugin.register';

/** The reason a result refuses the mod, or `undefined` when it lets it load. */
const refusalOf = (result: unknown): string | undefined =>
  isRecord(result) && typeof result.refuse === 'string' ? result.refuse : undefined;

/**
 * The rule of `plugin.register`: `{ refuse: reason }` with a string reason, or the result of
 * `next(e)` (judgment: the docs name no other answer, so another shape fails the hook like a throw).
 */
export function registerRules(book: RefusalBook): EventRuleBook {
  return {
    [REGISTER_EVENT]: {
      shapeProblem(result) {
        if (!isRecord(result))
          return `${REGISTER_EVENT} result: expected { refuse: reason } or next(e)`;
        return result.refuse === undefined || typeof result.refuse === 'string'
          ? undefined
          : `${REGISTER_EVENT} result: refuse must be a string`;
      },
      // A narrowed `provides` must be a subset of the one received (§10.19.5 item 2); the mod whose
      // hook is running is the one that narrowed it.
      changeProblem(original, changed) {
        const problem = providesProblem(original, changed);
        const running = book.running.at(-1);
        if (problem === undefined && running !== undefined && narrows(original, changed)) {
          book.narrowers.push(running);
        }
        return problem;
      },
      coreInput: withProvides,
    },
  };
}

const moduleCount = (provides: Record<string, unknown> | undefined): number =>
  Array.isArray(provides?.modules) ? provides.modules.length : 0;

/** Whether `changed` passes on fewer entries of `modules` than `original` holds. */
function narrows(original: unknown, changed: unknown): boolean {
  const before = isRecord(original) && isRecord(original.provides) ? original.provides : undefined;
  const after = isRecord(changed) && isRecord(changed.provides) ? changed.provides : undefined;
  return after !== undefined && moduleCount(after) < moduleCount(before);
}

/**
 * Who answered each refusal: the hook that returned a reason first (an inner hook finishes before the
 * hooks around it) is the refuser. `running` is the stack of mods whose `plugin.register` hook is
 * running, so a narrowing is told to the mod that passed it (`narrowers`).
 */
export interface RefusalBook {
  readonly refusers: Map<string, string>;
  readonly running: string[];
  readonly narrowers: string[];
}

export const createRefusalBook = (): RefusalBook => ({
  refusers: new Map(),
  running: [],
  narrowers: [],
});

/** Runs `run` with `plugin` on top of the book's stack of running hooks. */
async function whileRunning(
  book: RefusalBook,
  plugin: string,
  run: () => unknown,
): Promise<unknown> {
  book.running.push(plugin);
  try {
    return await run();
  } finally {
    book.running.pop();
  }
}

/** `hook` that notes, in `book`, the mod whose hook returned a refusal first. */
export function tagRefusals(
  hook: RegisteredHook,
  plugin: string,
  book: RefusalBook,
): RegisteredHook {
  if (hook.event !== REGISTER_EVENT) return hook;
  const note = (result: unknown): unknown => {
    const reason = refusalOf(result);
    if (reason !== undefined && !book.refusers.has(reason)) book.refusers.set(reason, plugin);
    return result;
  };
  const { catchRun } = hook;
  return {
    ...hook,
    run: (api, e, next) => whileRunning(book, plugin, () => hook.run(api, e, next)).then(note),
    ...(catchRun === undefined
      ? {}
      : {
          catchRun: (api, e, next) =>
            whileRunning(book, plugin, () => catchRun(api, e, next)).then(note),
        }),
  };
}

/** What `plugin.register` came to: the whole plugin refused, or the parts the mods left out. */
export type RegisterVerdict =
  | { readonly loads: false; readonly by: string; readonly reason: string }
  | {
      readonly loads: true;
      /** The entries the mods removed from `e.provides`, by kind; empty when nothing was left out. */
      readonly leftOut: Partial<Record<keyof PluginProvides, readonly unknown[]>>;
      /** The first mod whose hook narrowed `modules`, for `hooks module left out by <mod>`. */
      readonly by?: string;
    };

/** Asks the mods loaded so far whether the plugin of `event` may load, and which of its parts. */
export async function askToLoad(
  input: {
    readonly chain: ModChain;
    readonly book: RefusalBook;
    readonly signal: AbortSignal;
    readonly report: (report: HookReport) => void;
    readonly debug: (line: string) => void;
  },
  event: PluginRegisterInput,
): Promise<RegisterVerdict> {
  const { book } = input;
  book.refusers.clear();
  book.narrowers.length = 0;
  let kept: unknown = event.provides;
  const result = await input.chain.dispatch(REGISTER_EVENT, event, {
    origin: ENGINE_ORIGIN,
    signal: input.signal,
    core: (passed) => {
      kept = isRecord(passed) ? passed.provides : event.provides;
      return Promise.resolve({});
    },
    report: input.report,
    debug: input.debug,
  });
  const reason = refusalOf(result);
  if (reason !== undefined) {
    return { loads: false, by: book.refusers.get(reason) ?? 'a mod', reason };
  }
  const [by] = book.narrowers;
  return { loads: true, leftOut: leftOutOf(event.provides, kept), ...(by ? { by } : {}) };
}
