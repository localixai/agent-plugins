/**
 * What the recorder keeps while `bun test` runs: one entry per distinct call of an exported
 * function (module, name, arguments), with what it returned or threw and the requirement ids of
 * the `spec(...)` blocks the calling test ran under. Written to
 * `$RECORD_OUT/<pid>.jsonl` (`flush`, run by the preload after the tests); `scripts/rust/record.ts` merges the files into the vectors.
 */
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { encode, UNRECORDABLE } from './encode.ts';

type Outcome = { readonly result: unknown } | { readonly throws: string };

interface Entry {
  readonly module: string;
  readonly fn: string;
  readonly args: unknown;
  readonly async: boolean;
  outcomes: Outcome[];
  readonly spec: Set<string>;
}

/** Distinct calls kept per function: enough to cover its tests, small enough to commit. */
const PER_FUNCTION = 400;
/** A call whose arguments or answer take more JSON than this is not kept: a vector is a case. */
const MAX_CASE_CHARS = 16_384;

/**
 * Calls nested deeper than this inside recorded calls run unrecorded: a recursive reader of a
 * deep tree would otherwise encode its arguments at every level, and its outer call is the
 * vector that matters.
 */
const MAX_NESTING = 16;
let nesting = 0;

const entries = new Map<string, Entry>();
const perFunction = new Map<string, number>();
let context: readonly string[] = [];

export function runInContext<T>(ids: readonly string[], body: () => T): T {
  const previous = context;
  context = ids;
  try {
    const value = body();
    if (value instanceof Promise) {
      return value.finally(() => {
        context = previous;
      }) as T;
    }
    context = previous;
    return value;
  } catch (error) {
    context = previous;
    throw error;
  }
}

function entryFor(module: string, fn: string, args: unknown, isAsync: boolean): Entry | undefined {
  const text = JSON.stringify(args);
  if (text.length > MAX_CASE_CHARS) return undefined;
  const key = `${module}#${fn}#${text}`;
  const known = entries.get(key);
  if (known !== undefined) return known;
  const name = `${module}#${fn}`;
  const count = perFunction.get(name) ?? 0;
  if (count >= PER_FUNCTION) return undefined;
  perFunction.set(name, count + 1);
  const entry: Entry = { module, fn, args, async: isAsync, outcomes: [], spec: new Set() };
  entries.set(key, entry);
  return entry;
}

function note(
  entry: Entry | undefined,
  outcome: Outcome | undefined,
  ids: readonly string[],
): void {
  if (entry === undefined || outcome === undefined) return;
  for (const id of ids) entry.spec.add(id);
  const text = JSON.stringify(outcome);
  if (text.length > MAX_CASE_CHARS) return;
  if (!entry.outcomes.some((known) => JSON.stringify(known) === text)) entry.outcomes.push(outcome);
}

function resultOf(value: unknown): Outcome | undefined {
  const encoded = encode(value);
  return encoded === UNRECORDABLE ? undefined : { result: encoded };
}

function thrownOf(error: unknown): Outcome {
  return { throws: error instanceof Error ? error.message : String(error) };
}

/** One call of an exported function: `site` is `<module>#<name>`. */
export function recordCall(
  site: string,
  original: (...args: unknown[]) => unknown,
  self: unknown,
  args: unknown[],
): unknown {
  if (nesting >= MAX_NESTING) return original.apply(self, args);
  const [module = '', fn = ''] = site.split('#');
  const encoded = encode(args);
  const ids = context;
  if (encoded === UNRECORDABLE) return original.apply(self, args);
  let value: unknown;
  nesting += 1;
  try {
    value = original.apply(self, args);
  } catch (error) {
    note(entryFor(module, fn, encoded, false), thrownOf(error), ids);
    throw error;
  } finally {
    nesting -= 1;
  }
  if (value instanceof Promise) {
    return value.then(
      (settled: unknown) => {
        note(entryFor(module, fn, encoded, true), resultOf(settled), ids);
        return settled;
      },
      (error: unknown) => {
        note(entryFor(module, fn, encoded, true), thrownOf(error), ids);
        throw error;
      },
    );
  }
  note(entryFor(module, fn, encoded, false), resultOf(value), ids);
  return value;
}

/**
 * A wrapper the way the original looks from outside: its `name`, its `length` (a test may read
 * either) and its source text (a module may embed a function's text in generated code).
 */
export function dress(wrapper: Function, original: Function, name: string): void {
  const text = (): string =>
    Function.prototype.toString.call(original).replace(`${name}__recorded`, name);
  Object.defineProperty(wrapper, 'length', { value: original.length });
  Object.defineProperty(wrapper, 'name', { value: name });
  Object.defineProperty(wrapper, 'toString', { value: text });
}

/** The recording wrapper of an exported `const` function. */
export function wrap(
  module: string,
  name: string,
  original: (...args: unknown[]) => unknown,
): Function {
  const wrapper = function (this: unknown, ...args: unknown[]): unknown {
    return recordCall(`${module}#${name}`, original, this, args);
  };
  dress(wrapper, original, name);
  return wrapper;
}

/** Writes every entry so far; called after each test file, so it rewrites one file per process. */
export function flush(): void {
  const out = process.env['RECORD_OUT'];
  if (out === undefined) return;
  const lines: string[] = [];
  for (const entry of entries.values()) {
    if (entry.outcomes.length === 0) continue;
    lines.push(
      JSON.stringify({
        module: entry.module,
        fn: entry.fn,
        args: entry.args,
        async: entry.async,
        outcomes: entry.outcomes,
        spec: [...entry.spec].toSorted(),
      }),
    );
  }
  writeFileSync(join(out, `${process.pid}.jsonl`), lines.join('\n') + '\n');
}
