/**
 * The core's own values on the host's side of the wire (`wire.ts`): the objects its handles become,
 * the functions it answers, and which of them cross back as the core's own (`registry::Own`). Kept
 * apart from the tags of plain values, which need none of the host's tables.
 */

/**
 * The values of one call the host keeps by identity (`drive.ts`): what crosses as a number, and the
 * object a handle becomes.
 */
export interface HostValues {
  /** The number `value` crosses as. */
  keep(value: object): number;
  /** The value kept under `number`. */
  find(number: number): unknown;
  /** The object the host calls the core's handle `handle` through. */
  handle(handle: number, sync: readonly string[], async: readonly string[]): object;
  /** The promise that drives the core's task `task`, kept for a promise of the core's. */
  promise(task: number): Promise<unknown>;
  /**
   * Reads `other`'s values too: a core object given back holds the host values of the call that
   * made it (an output stage's scratch port), which the core may ask for in this call.
   */
  adopt(other: HostValues): void;
}

/** A tag as the core wrote it. */
export type Tagged = Readonly<Record<string, unknown>>;

/** How each tag is read back, by its key; `read` decodes what a collection holds. */
export type Reader = (
  tag: Tagged,
  read: (value: unknown) => unknown,
  hosts: HostValues | undefined,
) => { readonly value: unknown } | undefined;

/**
 * The objects the core's `$own` handles became, by identity: given back to the core, each crosses
 * as its handle, `{"$handle": h}`, and is the core's own object again (`registry::Own`). Another
 * handle given back stays a port, as the core reads it.
 */
const OWN = new WeakMap<object, OwnHandle>();

/** A core object's handle, and the host values of the call that made it. */
interface OwnHandle {
  readonly number: number;
  readonly hosts: HostValues;
}

/** The handle `value` crosses back as, when it is the core's own. */
export const ownOf = (value: unknown): OwnHandle | undefined =>
  (typeof value === 'object' && value !== null) || typeof value === 'function'
    ? OWN.get(value)
    : undefined;

/** Whether `member` is a method of the host's: a function that is not the core's own. */
export const isHostMethod = (member: unknown): boolean =>
  typeof member === 'function' && !OWN.has(member);

const names = (value: unknown): readonly string[] | undefined =>
  Array.isArray(value) && value.every((item) => typeof item === 'string') ? value : undefined;

/** A result with methods, as the object the host calls them through. */
const handle: Reader = (tag, _read, hosts) => {
  const [sync, async] = [names(tag.sync), names(tag.async)];
  return hosts !== undefined && typeof tag.$handle === 'number' && sync && async
    ? { value: hosts.handle(tag.$handle, sync, async) }
    : undefined;
};

/** A result with methods the host may give back: the object, remembered as the core's own. */
const own: Reader = (tag, _read, hosts) => {
  const number = tag.$own;
  const [sync, async] = [names(tag.sync), names(tag.async)];
  if (hosts === undefined || typeof number !== 'number' || !sync || !async) return undefined;
  const object = hosts.handle(number, sync, async);
  OWN.set(object, { number, hosts });
  return { value: object };
};

/** The function that calls the method `call` of the core's handle `number`, sync or async. */
function callOf(hosts: HostValues, number: number, async: boolean): unknown {
  const object = hosts.handle(number, async ? [] : ['call'], async ? ['call'] : []);
  const call: unknown = Reflect.get(object, 'call');
  return typeof call === 'function'
    ? (...args: readonly unknown[]): unknown => Reflect.apply(call, object, args)
    : undefined;
}

/** A function of the core's, as a function that calls its handle's method `call`. */
const coreFunction: Reader = (tag, _read, hosts) => {
  if (hosts === undefined || typeof tag.$call !== 'number') return undefined;
  const value = callOf(hosts, tag.$call, false);
  return typeof value === 'function' ? { value } : undefined;
};

/**
 * The async functions of the core's by their handle: the core answers the same object with the same
 * number while the host holds its function, and the host gives back that very function, bound to the
 * call that first made it (whose ports the core's object keeps).
 */
const CALLS = new Map<number, WeakRef<object>>();

const forgotten = new FinalizationRegistry<number>((number) => {
  if (CALLS.get(number)?.deref() === undefined) CALLS.delete(number);
});

/** An async function of the core's, remembered as the core's own so it crosses back as its handle. */
const coreAsyncFunction: Reader = (tag, _read, hosts) => {
  const number = tag.$asyncCall;
  if (hosts === undefined || typeof number !== 'number') return undefined;
  const known = CALLS.get(number)?.deref();
  if (known !== undefined && OWN.get(known)?.number === number) return { value: known };
  const value = callOf(hosts, number, true);
  if (typeof value !== 'function') return undefined;
  OWN.set(value, { number, hosts });
  CALLS.set(number, new WeakRef(value));
  forgotten.register(value, number);
  return { value };
};

/** The readers of the core's own values, by their tag's key. */
export const CORE_READERS: Readonly<Record<string, Reader>> = {
  $handle: handle,
  $own: own,
  $call: coreFunction,
  $asyncCall: coreAsyncFunction,
};
