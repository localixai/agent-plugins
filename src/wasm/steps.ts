/**
 * The steps of an export that waits on the host, as the core writes them
 * (`crates/agent-plugins/src/registry/steps.rs`), and the host's answers as it reads them; every
 * value through `wire.ts`.
 *
 * A step is `{"done": answer}`, `{"task": n, "needs": [{"id", "port", "method", "args"}]}` (the
 * task waits on these and on any asked before and not answered yet) or `{"rethrow": id}` (the
 * host threw for need `id`, so the export throws that). The answers are `{"answers": [{"id",
 * "result"} | {"id", "throws", "name"?}], "aborted": [n]}`, `throws` the message of what the host
 * threw and `name` an `Error`'s name (the core may catch it, `registry::catch_host`), `aborted`
 * the host's signals that aborted since the last step.
 */
import type { HostValues } from './core-values.ts';
import { type Answer, answerOf, decode, encode, parse, stringify } from './wire.ts';

/** One thing the core asks of the host: `method` of the value it keeps as `port`. */
export interface Need {
  readonly id: number;
  readonly port: number;
  readonly method: string;
  readonly args: readonly unknown[];
}

/** Where a task is: done, waiting on needs, or ended by a host's throw. */
export type Step =
  | { readonly done: Answer }
  | { readonly task: number; readonly needs: readonly Need[] }
  | { readonly rethrow: number };

/** The host's answer to a need: what the method gave, or that it threw (its message and name). */
export type Given =
  | { readonly id: number; readonly result: unknown }
  | { readonly id: number; readonly throws: string; readonly name?: string };

type Raw = Readonly<Record<string, unknown>>;

// The core writes only the shapes `Step` and `Need` name (steps.rs); JSON gives them as `unknown`.
// oxlint-disable-next-line typescript/no-unsafe-type-assertion -- as above.
const asRaw = (value: unknown): Raw => value as Raw;

const needOf = (raw: Raw, hosts: HostValues): Need => ({
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- steps.rs writes numbers, a string and an array here.
  ...(raw as Omit<Need, 'args'>),
  args: Array.isArray(raw.args) ? raw.args.map((arg) => decode(arg, hosts)) : [],
});

/** A step's JSON text read back, every value through `hosts`. */
export function decodeStep(text: string, hosts: HostValues): Step {
  const step = asRaw(parse(text));
  if ('done' in step) return { done: answerOf(asRaw(step.done), hosts) };
  if (typeof step.rethrow === 'number') return { rethrow: step.rethrow };
  const needs = Array.isArray(step.needs) ? step.needs : [];
  return {
    task: typeof step.task === 'number' ? step.task : -1,
    needs: needs.map((need) => needOf(asRaw(need), hosts)),
  };
}

/** The host's answers and its aborted signals as the JSON text `resume` reads. */
export const encodeAnswers = (
  answers: readonly Given[],
  aborted: readonly number[],
  hosts: HostValues,
): string =>
  stringify({
    answers: answers.map((given) =>
      'result' in given ? { id: given.id, result: encode(given.result, hosts) } : given,
    ),
    aborted,
  });
