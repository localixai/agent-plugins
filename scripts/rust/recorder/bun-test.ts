/**
 * `bun:test` as the recorded test files see it: the same API (a Proxy over each function, so
 * `each`, `skipIf`, `if`, `todo` and the rest are there), with `describe` keeping the names of the
 * blocks a test is registered in and `test`/`it` running each body in the context of the
 * requirement ids those names carry (`spec(id)` names its block `⟦id⟧`, tests/support/spec.ts).
 */
import * as real from 'bun:test';
import { runInContext } from './store.ts';

export * from 'bun:test';

const SPEC_ID = /⟦([^⟧]+)⟧/gu;
const blocks: string[] = [];

function currentIds(): readonly string[] {
  return blocks.flatMap((name) => [...name.matchAll(SPEC_ID)].map((match) => match[1] ?? ''));
}

type Body = (...args: never[]) => unknown;

/** A block body that keeps its name on the stack while it registers its tests. */
function inBlock(name: unknown, body: unknown): unknown {
  if (typeof body !== 'function') return body;
  return (...args: never[]) => {
    blocks.push(String(name));
    try {
      return (body as Body)(...args);
    } finally {
      blocks.pop();
    }
  };
}

/** A test body that runs in the context of the ids it was registered under. */
function bound(body: unknown): unknown {
  if (typeof body !== 'function') return body;
  const ids = currentIds();
  const run = body as Body;
  if (run.length === 1) return (done: never) => runInContext(ids, () => run(done));
  return (...args: never[]) => runInContext(ids, () => run(...args));
}

type Registrar = (name: unknown, body: unknown, ...rest: unknown[]) => unknown;

/** `register(name, body, ...)` with the body changed, and every method returning such a function too. */
function wrapped<T extends object>(
  original: T,
  change: (name: unknown, body: unknown) => unknown,
): T {
  return new Proxy(original, {
    apply(target, self, args: unknown[]) {
      const [name, body, ...rest] = args;
      return Reflect.apply(target as Registrar, self, [name, change(name, body), ...rest]);
    },
    get(target, key, receiver) {
      const value: unknown = Reflect.get(target, key, receiver);
      if (typeof value !== 'function') return value;
      return (...args: unknown[]) => {
        const made: unknown = Reflect.apply(value as Registrar, target, args);
        return typeof made === 'function' ? wrapped(made, change) : made;
      };
    },
  });
}

export const describe = wrapped(real.describe, inBlock);
export const test = wrapped(real.test, (_name, body) => bound(body));
export const it = wrapped(real.it, (_name, body) => bound(body));
