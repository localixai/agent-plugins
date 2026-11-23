import { elementsOf, equal, isObjectOrArray, matchesObject } from './expect-equal.ts';

/**
 * The checks of `expect(received)` (`Matchers`, claude-code/index.d.ts:21391 of 2.1.330), each a
 * rule over the received value and the arguments, and what a failure says it expected.
 */
export interface Check {
  readonly holds: (actual: unknown, args: readonly unknown[]) => boolean;
  readonly says: (args: readonly unknown[]) => string;
}

export const show = (value: unknown): string => {
  if (value === undefined) return 'undefined';
  if (typeof value === 'function') return '[function]';
  if (typeof value === 'bigint') return `${value}n`;
  if (typeof value === 'object' && value !== null && 'label' in value) return String(value.label);
  try {
    return JSON.stringify(value) ?? typeof value;
  } catch {
    return `[${typeof value}]`;
  }
};

const textOf = (value: unknown): string => (typeof value === 'string' ? value : show(value));

type Bound = number | bigint;
const isBound = (value: unknown): value is Bound =>
  typeof value === 'number' || typeof value === 'bigint';
const compared =
  (holds: (a: Bound, b: Bound) => boolean) =>
  (actual: unknown, [bound]: readonly unknown[]): boolean =>
    isBound(actual) && isBound(bound) && holds(actual, bound);

function textCheck(holds: (text: string, part: string) => boolean) {
  return (actual: unknown, [part]: readonly unknown[]): boolean =>
    typeof actual === 'string' && typeof part === 'string' && holds(actual, part);
}

function contains(actual: unknown, item: unknown): boolean {
  if (typeof actual === 'string') return actual.includes(textOf(item));
  return elementsOf(actual)?.some((element) => Object.is(element, item)) ?? false;
}

/** The value at `path` of `actual` (`a.b`, or the keys in order), and whether it is there. */
function at(actual: unknown, path: unknown): { readonly found: boolean; readonly value?: unknown } {
  const keys = Array.isArray(path) ? path.map(String) : String(path).split('.');
  let value = actual;
  for (const key of keys) {
    if (!isObjectOrArray(value) || !(key in value)) return { found: false };
    value = value[key];
  }
  return { found: true, value };
}

function hasProperty(actual: unknown, args: readonly unknown[]): boolean {
  const { found, value } = at(actual, args[0]);
  return found && (args.length < 2 || equal(value, args[1]));
}

/** Whether `error` is as `expected` describes it: a substring, a pattern, a class, or a message. */
export function errorMatches(error: unknown, expected: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  if (expected === undefined) return true;
  if (typeof expected === 'string') return message.includes(expected);
  if (expected instanceof RegExp) return expected.test(message);
  if (typeof expected === 'function') return error instanceof expected;
  return isObjectOrArray(expected) && message === expected.message;
}

/** Whether calling `actual` throws an error as `expected` describes it. */
function throws(actual: unknown, [expected]: readonly unknown[]): boolean {
  if (typeof actual !== 'function') return false;
  try {
    Reflect.apply(actual, undefined, []);
  } catch (error) {
    return errorMatches(error, expected);
  }
  return false;
}

const with1 = (verb: string) => (args: readonly unknown[]) => `${verb} ${show(args[0])}`;
const plain = (verb: string) => () => verb;

export const CHECKS = {
  toBe: { holds: (a, [b]) => Object.is(a, b), says: with1('be') },
  toEqual: { holds: (a, [b]) => equal(a, b), says: with1('equal') },
  toStrictEqual: { holds: (a, [b]) => equal(a, b, true), says: with1('strictly equal') },
  toMatchObject: { holds: (a, [b]) => matchesObject(a, b), says: with1('match the object') },
  toContain: { holds: (a, [b]) => contains(a, b), says: with1('contain') },
  toContainEqual: {
    holds: (a, [b]) => elementsOf(a)?.some((element) => equal(element, b)) ?? false,
    says: with1('contain an element equal to'),
  },
  toHaveLength: {
    holds: (a, [n]) =>
      isObjectOrArray(a) || typeof a === 'string' ? Reflect.get(Object(a), 'length') === n : false,
    says: with1('have length'),
  },
  toHaveProperty: { holds: hasProperty, says: (args) => `have the property ${show(args[0])}` },
  toBeUndefined: { holds: (a) => a === undefined, says: plain('be undefined') },
  toBeDefined: { holds: (a) => a !== undefined, says: plain('be defined') },
  toBeNull: { holds: (a) => a === null, says: plain('be null') },
  toBeTruthy: { holds: (a) => Boolean(a), says: plain('be truthy') },
  toBeFalsy: { holds: (a) => !a, says: plain('be falsy') },
  toBeNaN: { holds: (a) => Number.isNaN(a), says: plain('be NaN') },
  toBeGreaterThan: { holds: compared((a, b) => a > b), says: with1('be greater than') },
  toBeGreaterThanOrEqual: { holds: compared((a, b) => a >= b), says: with1('be at least') },
  toBeLessThan: { holds: compared((a, b) => a < b), says: with1('be less than') },
  toBeLessThanOrEqual: { holds: compared((a, b) => a <= b), says: with1('be at most') },
  toMatch: {
    holds: (a, [p]) =>
      typeof a === 'string' && (p instanceof RegExp ? p.test(a) : a.includes(textOf(p))),
    says: with1('match'),
  },
  toStartWith: { holds: textCheck((t, p) => t.startsWith(p)), says: with1('start with') },
  toEndWith: { holds: textCheck((t, p) => t.endsWith(p)), says: with1('end with') },
  toBeInstanceOf: {
    holds: (a, [c]) => typeof c === 'function' && a instanceof c,
    says: () => 'be an instance of the class',
  },
  toThrow: {
    holds: throws,
    says: (args) => (args[0] === undefined ? 'throw' : `throw an error matching ${show(args[0])}`),
  },
} as const satisfies Readonly<Record<string, Check>>;

export type CheckName = keyof typeof CHECKS;
