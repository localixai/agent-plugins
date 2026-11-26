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
 * Each crosses both ways; anything else is plain JSON.
 */

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

const toBase64 = (bytes: Uint8Array): string => {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
};

const fromBase64 = (text: string): Uint8Array => {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
};

/** A value JSON cannot spell, as its tag; `undefined` for one it can. */
function tagged(value: unknown): unknown {
  if (value === undefined) return UNDEFINED;
  if (typeof value === 'number') {
    if (Object.is(value, -0)) return { $number: '-0' };
    return Number.isFinite(value) ? undefined : { $number: String(value) };
  }
  if (value instanceof Set) return { $set: [...value] };
  if (value instanceof Map) return { $map: [...value] };
  if (value instanceof Uint8Array) return { $bytes: toBase64(value) };
  if (value instanceof RegExp) return { $regexp: value.source, flags: value.flags };
  return undefined;
}

/** Replaces each value JSON cannot spell with its tag, at every depth. */
function encode(value: unknown): unknown {
  const tag = tagged(value);
  // `undefined` and a number are tagged whole; a collection's tag holds items still to encode.
  if (tag !== undefined)
    return value === undefined || typeof value === 'number' ? tag : encode(tag);
  if (Array.isArray(value)) return value.map(encode);
  if (isRecord(value)) {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, encode(item)]));
  }
  return value;
}

/** A call's arguments as the JSON array the core reads. */
export const encodeArgs = (args: readonly unknown[]): string => stringify(encode(args));

type Tagged = Readonly<Record<string, unknown>>;

/** How each tag is read back, by its key; `read` decodes what a collection holds. */
type Reader = (
  tag: Tagged,
  read: (value: unknown) => unknown,
) => { readonly value: unknown } | undefined;

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
};

/** The members a tag has besides its key: only `$regexp` has one, its `flags`. */
const MEMBERS: Readonly<Record<string, number>> = { $regexp: 2 };

/** A tag read back as its value (`{ value }`), or `undefined` when `value` is not a tag. */
function untagged(
  value: unknown,
  read: (item: unknown) => unknown,
): { readonly value: unknown } | undefined {
  if (!isRecord(value)) return undefined;
  const keys = Object.keys(value);
  const key = keys.find((name) => name.startsWith('$'));
  if (key === undefined || keys.length !== (MEMBERS[key] ?? 1)) return undefined;
  return READERS[key]?.(value, read);
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
function decode(value: unknown): unknown {
  const tag = untagged(value, decode);
  if (tag !== undefined) return tag.value;
  if (Array.isArray(value)) return value.map(decode);
  if (isRecord(value)) {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, decode(item)]));
  }
  return value;
}

/** An answer's JSON text read back, every tag as its value. */
export const decodeAnswer = (text: string): Answer => {
  // The core's answer is one of the three shapes `Answer` names (registry.rs, `call`).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- the core writes only an object here.
  const answer = parse(text) as Readonly<Record<string, unknown>>;
  if ('result' in answer) return { result: decode(answer.result) };
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- `throws` or `fault`, as registry.rs writes them.
  return answer as Answer;
};
