/**
 * The host's loop over the core (docs/RUST.md, "The boundary"): a call starts its export, does
 * what each step needs (a method of a port, a callback) and gives the core the answers, until the
 * step is done. Nothing crosses but numbers and data: the host's values stay here, in the call's
 * table (`hosts.ts`).
 *
 * `callSync` answers at once, for a function the TypeScript declares synchronous: with no host
 * value among the arguments it is one call of the core; with a callback, or for a handle's method
 * (which may use the ports of the call that made the handle), each need is answered as it comes,
 * and a callback that returns a promise is a bug of the wrapper. `callAsync` awaits
 * each need, starting it the moment the core asks, so needs asked together run together, as
 * `Promise.all` in the TypeScript. What a host value throws, the call throws, the same value,
 * unless the core catches it (`registry::catch_host`); the value stays here by its need either way.
 *
 * A promise of the core's in an answer (`{"$promise": {"task": t}}`) is driven by the same loop as
 * `callAsync`, at once, so its first needs start before the answer that holds it is returned, as
 * the TypeScript's `async` function ran to its first `await` when it was called. Its needs are
 * answered with the host values of the call that answered it. The task ends when the promise
 * settles; a loop that can never go on (it waits on a host promise nothing holds) is collected,
 * and its task cancelled with it.
 */
import { type HostTable, type Makers, hostTable } from './hosts.ts';
import { callText, cancel, generation, release, resumeText, startText } from './instance.ts';
import { type Given, type Need, type Step, decodeStep, encodeAnswers } from './steps.ts';
import { type Answer, decodeAnswer, encodeArgs } from './wire.ts';

/**
 * The result of `name`'s answer, or the `Error` with the message the TypeScript threw. A fault (an
 * unknown name, arguments that do not decode) is a bug of the package.
 *
 * `T` is the wrapper's declared return type: the core answers the export's TypeScript type, which
 * the package's tests hold every wrapper to, so this is the one place the type is taken on trust.
 */
// oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- T is the wrapper's declared return type (above).
function resultOf<T>(name: string, answer: Answer): T {
  // The core's result is the export's declared type (above); JSON gives it as `unknown`.
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- the one place the type is taken on trust (above).
  if ('result' in answer) return answer.result as T;
  if ('throws' in answer) throw new Error(answer.throws);
  throw new Error(
    `agent-plugins: ${name}: ${answer.fault} (a bug in agent-plugins; please report it)`,
  );
}

type Done = Exclude<Step, { readonly needs: readonly Need[] }>;

// oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- as resultOf's T.
function finished<T>(name: string, step: Done, thrown: ReadonlyMap<number, unknown>): T {
  // What the host threw for the need, the same value, as the TypeScript's await threw it.
  // oxlint-disable-next-line typescript/only-throw-error -- the host's own value, whatever it is.
  if ('rethrow' in step) throw thrown.get(step.rethrow);
  return resultOf<T>(name, step.done);
}

/** A throw as the core reads it (`registry::Thrown`): its message, and an `Error`'s `name`. */
const thrownAs = (id: number, error: unknown): Given =>
  error instanceof Error
    ? { id, throws: error.message, name: error.name }
    : { id, throws: String(error) };

const isThenable = (value: unknown): boolean =>
  typeof value === 'object' && value !== null && typeof Reflect.get(value, 'then') === 'function';

/**
 * Does a need: calls the callback, or the port's method, with its arguments. A need `.<name>`
 * reads the member instead, `undefined` when it is absent: a port's property (`ScanFolder.dir`),
 * or a member of a record that crossed as a port because it holds a function.
 */
function invoke(need: Need, hosts: HostTable): unknown {
  const target = hosts.find(need.port);
  if (typeof target === 'function') return Reflect.apply(target, undefined, need.args);
  const isObject = typeof target === 'object' && target !== null;
  if (need.method.startsWith('.'))
    return isObject ? Reflect.get(target, need.method.slice(1)) : undefined;
  const method: unknown = isObject ? Reflect.get(target, need.method) : undefined;
  if (typeof method !== 'function') {
    throw new TypeError(`agent-plugins: host value ${need.port} has no method ${need.method}`);
  }
  return Reflect.apply(method, target, need.args);
}

/**
 * The need `$hold` (`registry::detach`): its argument is a promise of the core's, which reading the
 * need already started, answered as it is, not awaited, so the core holds it as the host's own.
 */
const held = (need: Need): Given | undefined =>
  need.method === '$hold' ? { id: need.id, result: need.args[0] } : undefined;

function answerSync(need: Need, hosts: HostTable, thrown: Map<number, unknown>): Given {
  const hold = held(need);
  if (hold !== undefined) return hold;
  try {
    const result = invoke(need, hosts);
    if (isThenable(result)) {
      throw new TypeError(`agent-plugins: ${need.method} gave a promise to a synchronous call`);
    }
    return { id: need.id, result };
  } catch (error) {
    thrown.set(need.id, error);
    return thrownAs(need.id, error);
  }
}

async function answerAsync(
  need: Need,
  hosts: HostTable,
  thrown: Map<number, unknown>,
): Promise<Given> {
  const hold = held(need);
  if (hold !== undefined) return hold;
  try {
    return { id: need.id, result: await invoke(need, hosts) };
  } catch (error) {
    thrown.set(need.id, error);
    return thrownAs(need.id, error);
  }
}

const resumed = (task: number, answers: readonly Given[], hosts: HostTable): Step =>
  decodeStep(resumeText(task, encodeAnswers(answers, hosts.aborted(), hosts)), hosts);

const MAKERS: Makers = { handle: handleObject, promise: corePromise };

const handles = new FinalizationRegistry<readonly [number, number]>(([handle, madeIn]) => {
  release(handle, madeIn);
});

/** A core handle as an object with its methods, each a call of `<handle>#<method>`. */
function handleObject(
  handle: number,
  methods: { readonly sync: readonly string[]; readonly async: readonly string[] },
  hosts: HostTable,
): object {
  const name = (method: string): string => `${String(handle)}#${method}`;
  const object = Object.fromEntries([
    ...methods.sync.map((method) => [
      method,
      (...args: readonly unknown[]): unknown => callSync(name(method), args, hosts),
    ]),
    ...methods.async.map((method) => [
      method,
      (...args: readonly unknown[]): Promise<unknown> => callAsync(name(method), args, hosts),
    ]),
  ]);
  handles.register(object, [handle, generation()]);
  return object;
}

/** `name` called with `args`, answered at once (above); `parent` for a handle's method. */
// oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- as resultOf's T.
export function callSync<T>(name: string, args: readonly unknown[], parent?: HostTable): T {
  const hosts = hostTable(MAKERS, parent);
  const text = encodeArgs(args, hosts);
  // A handle's method may ask for the host values of the call that made it, so it takes the loop.
  if (hosts.empty() && parent === undefined) {
    return resultOf<T>(name, decodeAnswer(callText(name, text), hosts));
  }
  const thrown = new Map<number, unknown>();
  let step = decodeStep(startText(name, text), hosts);
  while ('needs' in step) {
    const answers = step.needs.map((need) => answerSync(need, hosts, thrown));
    step = resumed(step.task, answers, hosts);
  }
  return finished<T>(name, step, thrown);
}

/** Runs a task from its `first` step until it is done: each need started as the core asks. */
// oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- as resultOf's T.
async function driven<T>(name: string, hosts: HostTable, first: () => Step): Promise<T> {
  const thrown = new Map<number, unknown>();
  const running = new Map<number, Promise<Given>>();
  let step = first();
  while ('needs' in step) {
    for (const need of step.needs) running.set(need.id, answerAsync(need, hosts, thrown));
    if (running.size === 0) throw new Error(`agent-plugins: ${name} waits on nothing (a bug)`);
    // One answer at a time, the first to settle: the core runs on as soon as it can.
    // oxlint-disable-next-line no-await-in-loop -- each step depends on the answer before it.
    const given = await Promise.race(running.values());
    running.delete(given.id);
    step = resumed(step.task, [given], hosts);
  }
  return finished<T>(name, step, thrown);
}

/** `name` called with `args` as a task the host drives (above); `parent` for a handle's method. */
// oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- as resultOf's T.
export const callAsync = <T>(
  name: string,
  args: readonly unknown[],
  parent?: HostTable,
): Promise<T> => {
  const hosts = hostTable(MAKERS, parent);
  return driven<T>(name, hosts, () => decodeStep(startText(name, encodeArgs(args, hosts)), hosts));
};

/** The core's tasks whose loop was collected before they settled, by the instance that keeps them. */
const loops = new FinalizationRegistry<readonly [number, number]>(([task, madeIn]) => {
  cancel(task, madeIn);
});

/**
 * The promise of the core's task `task` (above). `lease` lives as long as the loop can go on: it is
 * used again once the loop ends, so the loop's suspended frame holds it.
 */
async function corePromise(task: number, parent: HostTable): Promise<unknown> {
  const hosts = hostTable(MAKERS, parent);
  const lease = {};
  loops.register(lease, [task, generation()], lease);
  try {
    return await driven(`the core's promise ${String(task)}`, hosts, () =>
      resumed(task, [], hosts),
    );
  } finally {
    loops.unregister(lease);
  }
}
