/**
 * The values that cross into the Rust core and back (`crates/agent-plugins/src/registry/wire.rs`):
 * JSON, with what JSON has no spelling for tagged, as the recorder of the vectors tags it:
 *
 * - `undefined`: `{"$undefined": true}`, at the top, as an array item or an object member;
 * - `NaN`, `Infinity`, `-Infinity` and `-0`: `{"$number": "NaN" | "Infinity" | "-Infinity" | "-0"}`
 *   (`JSON.stringify` writes the first three as `null` and `-0` as `0`);
 * - a `Set`: `{"$set": [items]}`; a `Map`: `{"$map": [[key, value]…]}`;
 * - a `Uint8Array`: `{"$bytes": "<base64>"}`; a `RegExp`: `{"$regexp": source, "flags": flags}`.
 *
 * Each crosses both ways; anything else is plain JSON. For an export that waits on the host
 * (`call.async`, or a callback given to `call`), what cannot be copied is kept by identity in the
 * call's `HostValues` and crosses as its number, both ways:
 *
 * - a port (an object with methods): `{"$port": n}`; a function: `{"$fn": n}`;
 * - an `AbortSignal`: `{"$signal": n, "aborted": bool}` (`n` is `null` for a signal the core
 *   made, which arrives as a new one);
 * - a promise of the host's: `{"$promise": n}` (the core awaits it by the need `then`);
 * - from the core only, a result with methods: `{"$handle": h, "sync": [names], "async": [names]}`,
 *   or `{"$own": h, …}` for one the host may give back, which then crosses as `{"$handle": h}`;
 *   a function of the core's (a timer's `fire`): `{"$call": h}`, a function that calls the
 *   handle's one method, `call`, or `{"$asyncCall": h}` for one whose `call` returns a promise
 *   (a skill's `readBody`), which given back crosses as `{"$handle": h}`; and a promise of the
 *   core's: `{"$promise": {"task": t}}`, a promise that drives task `t` (`drive.ts`).
 */

import { fromBase64, toBase64 } from './base64.ts';
import { CORE_READERS, isHostMethod, ownOf, type HostValues, type Reader } from './core-values.ts';

const UNDEFINED = { $undefined: true } as const;

/**
 * JSON's own functions, taken when the bridge loads: crossing the boundary is not the package
 * parsing a document, so a meter that wraps `JSON.parse` later (docs/TESTING.md, "Counts") does
 * not count it.
 */
const { parse, stringify } = JSON;

/** The numbers JSON has no text for, by their tag's name. */
const NUMBERS: ReadonlyMap<string, number> = new Map([
  ['NaN', Number.NaN],
  ['Infinity', Number.POSITIVE_INFINITY],
  ['-Infinity', Number.NEGATIVE_INFINITY],
  ['-0', -0],
]);

const isRecord = (value: unknown): value is Readonly<Record<string, unknown>> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

/**
 * Whether `value` is a port: an object with a method, its own or its class's. A function of the
 * core's own (`$asyncCall`, a scanned skill's `readBody`) is no method: it crosses as its handle, so
 * the record that holds it is data.
 */
function hasMethods(value: object): boolean {
  if (Object.values(value).some(isHostMethod)) return true;
  const prototype: unknown = Object.getPrototypeOf(value);
  return prototype !== null && prototype !== Object.prototype;
}

/** A host value as its tag, kept in `hosts`; `undefined` for data. */
function hostTagged(value: unknown, hosts: HostValues): unknown {
  if (typeof value === 'function') return { $fn: hosts.keep(value) };
  if (value instanceof Promise) return { $promise: hosts.keep(value) };
  if (value instanceof AbortSignal) return { $signal: hosts.keep(value), aborted: value.aborted };
  return isRecord(value) && hasMethods(value) ? { $port: hosts.keep(value) } : undefined;
}

/** A number JSON has no text for, as its tag; `undefined` for one it has. */
const numberTag = (value: number): unknown => {
  if (Object.is(value, -0)) return { $number: '-0' };
  return Number.isFinite(value) ? undefined : { $number: String(value) };
};

/** A value JSON cannot spell, as its tag; `undefined` for one it can. */
function tagged(value: unknown, hosts: HostValues | undefined): unknown {
  if (value === undefined) return UNDEFINED;
  const own = ownOf(value);
  if (own !== undefined) return { $handle: own };
  if (typeof value === 'number') return numberTag(value);
  if (value instanceof Set) return { $set: [...value] };
  if (value instanceof Map) return { $map: [...value] };
  if (value instanceof Uint8Array) return { $bytes: toBase64(value) };
  if (value instanceof RegExp) return { $regexp: value.source, flags: value.flags };
  return hosts === undefined ? undefined : hostTagged(value, hosts);
}

/** Replaces each value JSON cannot spell with its tag, at every depth; host values go to `hosts`. */
export function encode(value: unknown, hosts?: HostValues): unknown {
  const tag = tagged(value, hosts);
  // `undefined` and a number are tagged whole; a collection's tag holds items still to encode.
  if (tag !== undefined)
    return value === undefined || typeof value === 'number' ? tag : encode(tag, hosts);
  if (Array.isArray(value)) return value.map((item) => encode(item, hosts));
  if (isRecord(value)) {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, encode(item, hosts)]),
    );
  }
  return value;
}

/** A call's arguments as the JSON array the core reads. */
export const encodeArgs = (args: readonly unknown[], hosts?: HostValues): string =>
  stringify(encode(args, hosts));

/** A host value the core names by its number. */
const kept: Reader = (tag, _read, hosts) => {
  const number = tag.$port ?? tag.$fn ?? tag.$signal ?? tag.$promise;
  return hosts !== undefined && typeof number === 'number'
    ? { value: hosts.find(number) }
    : undefined;
};

/** A signal: the host's own by its number, or a new one for a signal the core made. */
const signal: Reader = (tag, read, hosts) => {
  if (tag.$signal !== null) return kept(tag, read, hosts);
  return hosts !== undefined && typeof tag.aborted === 'boolean'
    ? { value: tag.aborted ? AbortSignal.abort() : new AbortController().signal }
    : undefined;
};

/** A promise: the host's own by its number, or one that drives the core's task. */
const promise: Reader = (tag, read, hosts) => {
  if (typeof tag.$promise === 'number') return kept(tag, read, hosts);
  const task: unknown = isRecord(tag.$promise) ? tag.$promise.task : undefined;
  return hosts !== undefined && typeof task === 'number'
    ? { value: hosts.promise(task) }
    : undefined;
};

const READERS: Readonly<Record<string, Reader | undefined>> = {
  $undefined: (tag) => (tag.$undefined === true ? { value: undefined } : undefined),
  $number: (tag) =>
    typeof tag.$number === 'string' && NUMBERS.has(tag.$number)
      ? { value: NUMBERS.get(tag.$number) }
      : undefined,
  $set: (tag, read) =>
    Array.isArray(tag.$set) ? { value: new Set(tag.$set.map(read)) } : undefined,
  $map: (tag, read) =>
    Array.isArray(tag.$map)
      ? // A `$map` holds `[key, value]` pairs: the core writes no other shape under this tag.
        // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- as above.
        { value: new Map((read(tag.$map) as readonly (readonly [unknown, unknown])[]).values()) }
      : undefined,
  $bytes: (tag) => (typeof tag.$bytes === 'string' ? { value: fromBase64(tag.$bytes) } : undefined),
  $regexp: (tag) =>
    typeof tag.$regexp === 'string' && typeof tag.flags === 'string'
      ? { value: new RegExp(tag.$regexp, tag.flags) }
      : undefined,
  $port: kept,
  $fn: kept,
  $signal: signal,
  ...CORE_READERS,
  $promise: promise,
};

/** How many members a tag has with its key, when more than one. */
const MEMBERS: Readonly<Record<string, number>> = {
  $regexp: 2,
  $signal: 2,
  $handle: 3,
  $own: 3,
};

/** A tag read back as its value (`{ value }`), or `undefined` when `value` is not a tag. */
function untagged(
  value: unknown,
  read: (item: unknown) => unknown,
  hosts: HostValues | undefined,
): { readonly value: unknown } | undefined {
  if (!isRecord(value)) return undefined;
  const keys = Object.keys(value);
  const key = keys.find((name) => name.startsWith('$'));
  if (key === undefined || keys.length !== (MEMBERS[key] ?? 1)) return undefined;
  return READERS[key]?.(value, read, hosts);
}

/** What the core answered: a result, the message of what the function throws, or a fault. */
export type Answer =
  | { readonly result: unknown }
  | { readonly throws: string }
  | { readonly fault: string };

/**
 * A parsed value with every tag read back, at every depth. A walk, not a `JSON.parse` reviver: a
 * reviver that returns `undefined` deletes the member, and `{"result": undefined}` (a function
 * that returns nothing) must keep it, as must an object member the TypeScript sets to `undefined`.
 */
export function decode(value: unknown, hosts?: HostValues): unknown {
  const read = (item: unknown): unknown => decode(item, hosts);
  const tag = untagged(value, read, hosts);
  if (tag !== undefined) return tag.value;
  if (Array.isArray(value)) return value.map(read);
  if (isRecord(value)) {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, read(item)]));
  }
  return value;
}

/** A parsed answer with its result read back (`registry.rs`, `call`). */
export function answerOf(answer: Readonly<Record<string, unknown>>, hosts?: HostValues): Answer {
  if ('result' in answer) return { result: decode(answer.result, hosts) };
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- `throws` or `fault`, as registry.rs writes them.
  return answer as Answer;
}

/** An answer's JSON text read back, every tag as its value. */
export const decodeAnswer = (text: string, hosts?: HostValues): Answer =>
  // The core's answer is one of the three shapes `Answer` names (registry.rs, `call`).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- the core writes only an object here.
  answerOf(parse(text) as Readonly<Record<string, unknown>>, hosts);

/** JSON's own `parse` and `stringify`, for the steps (`steps.ts`): see above. */
export { parse, stringify };
