/**
 * How `expect` of the test kit compares values (`Matchers`, `Matching`, claude-code/index.d.ts:21391
 * of 2.1.330): `toEqual` by structure with properties holding `undefined` ignored, `toStrictEqual`
 * with them and prototypes counted, and the asymmetric matchers standing inside an expected value.
 */

const ASYMMETRIC: unique symbol = Symbol('asymmetric matcher');

/** A matcher that stands inside an expected value (`expect.any(Number)`), matching by its rule. */
export interface Asymmetric {
  readonly [ASYMMETRIC]: true;
  readonly matches: (actual: unknown) => boolean;
  /** What a failure shows for it, as `expect.any(Number)`. */
  readonly label: string;
}

export const asymmetric = (label: string, matches: (actual: unknown) => boolean): Asymmetric => ({
  [ASYMMETRIC]: true,
  matches,
  label,
});

const isAsymmetric = (value: unknown): value is Asymmetric =>
  typeof value === 'object' && value !== null && ASYMMETRIC in value;

/** Any object, arrays included: both compare by their keys. */
export const isObjectOrArray = (value: unknown): value is Readonly<Record<string, unknown>> =>
  typeof value === 'object' && value !== null;

const definedKeys = (value: object): readonly string[] =>
  Object.keys(value).filter((key) => Reflect.get(value, key) !== undefined);

function sameKeys(a: object, b: object, strict: boolean): readonly string[] | undefined {
  const keysA = strict ? Object.keys(a) : definedKeys(a);
  const keysB = strict ? Object.keys(b) : definedKeys(b);
  if (keysA.length !== keysB.length || !keysA.every((key) => keysB.includes(key))) return undefined;
  return keysA;
}

/** Whether `actual` equals `expected`: `toEqual`, or with `strict` `toStrictEqual`. */
export function equal(actual: unknown, expected: unknown, strict = false): boolean {
  if (isAsymmetric(expected)) return expected.matches(actual);
  if (Object.is(actual, expected)) return true;
  if (!isObjectOrArray(actual) || !isObjectOrArray(expected)) return false;
  if (Array.isArray(actual) !== Array.isArray(expected)) return false;
  if (strict && Object.getPrototypeOf(actual) !== Object.getPrototypeOf(expected)) return false;
  const keys = sameKeys(actual, expected, strict);
  return keys !== undefined && keys.every((key) => equal(actual[key], expected[key], strict));
}

/** Whether `actual` holds every property of `expected`, nested objects compared the same way. */
export function matchesObject(actual: unknown, expected: unknown): boolean {
  if (isAsymmetric(expected)) return expected.matches(actual);
  if (!isObjectOrArray(expected) || Array.isArray(expected)) return equal(actual, expected);
  if (!isObjectOrArray(actual)) return false;
  return Object.entries(expected).every(([key, value]) => matchesObject(actual[key], value));
}

const isIterable = (value: object): value is Iterable<unknown> =>
  typeof Reflect.get(value, Symbol.iterator) === 'function';

/** The elements of an iterable received (a string's characters aside), or `undefined`. */
export function elementsOf(value: unknown): readonly unknown[] | undefined {
  if (typeof value === 'string' || !isObjectOrArray(value) || !isIterable(value)) return undefined;
  return [...value];
}
