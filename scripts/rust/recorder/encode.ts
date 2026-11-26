/**
 * A value as the vectors hold it: JSON, with the few JavaScript values JSON lacks spelled as
 * tagged objects the Rust side reads back (`crates/agent-plugins/tests/support/vectors.rs`).
 * Anything that is not plain data (a function, a class instance, a promise, a cycle) makes the
 * whole call unrecordable: `encode` returns `UNRECORDABLE` and the call is skipped.
 */

export const UNRECORDABLE: unique symbol = Symbol('unrecordable');
type Encoded = null | boolean | number | string | Encoded[] | { [key: string]: Encoded };
type Result = Encoded | typeof UNRECORDABLE;

const MAX_DEPTH = 64;

function isPlainObject(value: object): boolean {
  const prototype: unknown = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function encodeNumber(value: number): Encoded {
  if (Number.isFinite(value)) return Object.is(value, -0) ? { $number: '-0' } : value;
  return { $number: String(value) };
}

function encodeList(values: readonly unknown[], seen: Set<object>, depth: number): Result {
  const out: Encoded[] = [];
  for (const item of values) {
    const encoded = encodeAt(item, seen, depth + 1);
    if (encoded === UNRECORDABLE) return UNRECORDABLE;
    out.push(encoded);
  }
  return out;
}

/** A getter is never called: reading it could count as a read the test then checks. */
function encodeRecord(value: object, seen: Set<object>, depth: number): Result {
  const out: { [key: string]: Encoded } = {};
  for (const [key, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(value))) {
    if (!descriptor.enumerable) continue;
    if (!('value' in descriptor)) return UNRECORDABLE;
    const item: unknown = descriptor.value;
    if (item === undefined) continue;
    const encoded = encodeAt(item, seen, depth + 1);
    if (encoded === UNRECORDABLE) return UNRECORDABLE;
    // Defined, not assigned: `out['__proto__'] = …` would set the prototype and lose the member.
    Object.defineProperty(out, key, {
      value: encoded,
      enumerable: true,
      writable: true,
      configurable: true,
    });
  }
  return out;
}

function encodeObject(value: object, seen: Set<object>, depth: number): Result {
  if (Array.isArray(value)) {
    return Object.getPrototypeOf(value) === Array.prototype
      ? encodeList(value, seen, depth)
      : UNRECORDABLE;
  }
  if (value instanceof Map) {
    const entries = encodeList([...value.entries()], seen, depth);
    return entries === UNRECORDABLE ? entries : { $map: entries };
  }
  if (value instanceof Set) {
    const items = encodeList([...value.values()], seen, depth);
    return items === UNRECORDABLE ? items : { $set: items };
  }
  if (value instanceof RegExp) return { $regexp: value.source, flags: value.flags };
  if (value instanceof Uint8Array) return { $bytes: Buffer.from(value).toString('base64') };
  return isPlainObject(value) ? encodeRecord(value, seen, depth) : UNRECORDABLE;
}

function encodeAt(value: unknown, seen: Set<object>, depth: number): Result {
  if (depth > MAX_DEPTH) return UNRECORDABLE;
  if (value === undefined) return { $undefined: true };
  if (value === null || typeof value === 'boolean' || typeof value === 'string') return value;
  if (typeof value === 'number') return encodeNumber(value);
  if (typeof value !== 'object') return UNRECORDABLE;
  if (seen.has(value)) return UNRECORDABLE;
  seen.add(value);
  const encoded = encodeObject(value, seen, depth);
  seen.delete(value);
  return encoded;
}

export function encode(value: unknown): Result {
  return encodeAt(value, new Set(), 0);
}
