import {
  API_EVENT_NAMES,
  CLASSIC_EVENT_NAMES,
  ENGINE_EVENT_NAMES,
  STREAM_EVENTS,
  type AnyModEventName,
} from '../../protocol/mods-events.ts';
import type { RegisteredHook } from '../../mods/types.ts';
import { ExpectationError } from './expect.ts';
import { noImplementation } from './fake-ports.ts';
import { stubAdmits } from './stub-matcher.ts';
import type { ModMatcher } from '../../protocol/mods-matcher.ts';

/**
 * The stubs of a test (plugins/mods/test.md, "Stub what Claude Code would answer"): hooks the test
 * registers with `on` that answer in Claude Code's place. A stub for an event Claude Code fires
 * answers at the end of the chain; a stub for a mods API call answers the call of a mod, whose
 * hooks run after it, as the hook of a mod before it would. Several stubs for one name run in the
 * order they were registered, each passing to the next with `next(e)`.
 */

/** What the test's `on` takes: a hook that answers, plain or (for a stream) an async generator. */
export type StubHook = ($: unknown, e: never, next: (e: unknown) => unknown) => unknown;

const NAMES: ReadonlySet<string> = new Set<string>([
  ...ENGINE_EVENT_NAMES,
  ...CLASSIC_EVENT_NAMES,
  ...API_EVENT_NAMES,
]);
const ENGINE: ReadonlySet<string> = new Set<string>([
  ...ENGINE_EVENT_NAMES,
  ...CLASSIC_EVENT_NAMES,
]);
const STREAMS: ReadonlySet<string> = new Set(STREAM_EVENTS);

export const isKnownEvent = (name: string): name is AnyModEventName => NAMES.has(name);
/** Whether a mod can call it: a mods API call, whether or not Claude Code fires an event of that name too. */
const isCall = (name: string): boolean => API_EVENT_NAMES.includes(name);

/** One stub: the hook and the matcher it was registered with. */
interface Stub {
  readonly hook: StubHook;
  readonly matcher: ModMatcher | undefined;
}

export interface StubBook {
  add(name: AnyModEventName, hook: StubHook, matcher?: ModMatcher): void;
  /** Whether a stub of `name` answers `e`: its matcher lets it through. */
  admits(name: string, e: unknown): boolean;
  /** The names that have a stub. */
  names(): readonly string[];
  /** The answer of the stubs of `name` to `e`, in Claude Code's place. */
  answer(name: string, e: unknown, $: unknown): Promise<unknown> | AsyncGenerator<unknown, unknown>;
  /** The hooks to register ahead of the mods, so the calls of mods reach the stubs. */
  hooks($: () => unknown): readonly RegisteredHook[];
}

const missing = (name: string): Promise<never> => Promise.reject(new Error(noImplementation(name)));

export function createStubBook(): StubBook {
  const stubs = new Map<string, Stub[]>();
  /** The stubs of `name` from `index` on whose matcher lets `e` through, in the order registered. */
  const from = (name: string, e: unknown, index: number): number =>
    (stubs.get(name) ?? []).findIndex((stub, at) => at >= index && stubAdmits(stub.matcher, e));
  /** The first stub of `name` at or after `index` that answers `e`, with `next` leading to the one after it, then to `no implementation`. */
  const run = (name: string, e: unknown, $: unknown, index: number): unknown => {
    const at = from(name, e, index);
    const hook = at < 0 ? undefined : stubs.get(name)?.[at]?.hook;
    if (!hook) return STREAMS.has(name) ? emptyStream(name) : missing(name);
    return Reflect.apply(hook, undefined, [
      $,
      // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- the stub was written for this event's input, which the chain hands it as it is
      e as never,
      (next: unknown) => run(name, next, $, at + 1),
    ]);
  };
  return {
    add(name, hook, matcher) {
      stubs.set(name, [...(stubs.get(name) ?? []), { hook, matcher }]);
    },
    admits: (name, e) => from(name, e, 0) >= 0,
    names: () => [...stubs.keys()],
    answer: (name, e, $) =>
      // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- a stub of a stream is an async generator, any other is a promise or a value
      run(name, e, $, 0) as Promise<unknown> | AsyncGenerator<unknown, unknown>,
    hooks: ($) =>
      [...stubs.keys()].filter(isCall).map((name): RegisteredHook => {
        const answer = (e: unknown, index: number): unknown => run(name, e, $(), index);
        return {
          event: name,
          isGenerator: STREAMS.has(name),
          // A stub answers what a mod calls; what Claude Code fires reaches it as `core`, after the mods.
          run: (_api, e, next) =>
            ENGINE.has(name) && next.origin.plugin === 'engine' ? next(e) : answer(e, 0),
        };
      }),
  };
}

/** A stream that ends at once with nothing: a stub of a stream event that nobody wrote. */
async function* emptyStream(name: string): AsyncGenerator<unknown, unknown> {
  yield* [];
  return missing(name);
}

/** A stub that notes a failed `expect` in the test, so the test fails though the engine skips the hook (test.md: "in the test's store.set hook"). */
export function noting(name: string, hook: StubHook, fail: (message: string) => void): StubHook {
  const note = (error: unknown): never => {
    if (error instanceof ExpectationError) fail(`in the test's ${name} hook: ${error.message}`);
    throw error;
  };
  return (...args) => {
    try {
      const out = Reflect.apply(hook, undefined, args);
      return out instanceof Promise ? out.catch(note) : out;
    } catch (error) {
      return note(error);
    }
  };
}
