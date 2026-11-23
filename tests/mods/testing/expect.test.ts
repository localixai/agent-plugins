/**
 * `expect` of the test kit (module `claude-code/testing`).
 * Agent Plugins Extended §10.12 (reference.md: "The test kit's `expect(value)` MUST offer the
 * assertions of the declarations' `Matchers` type ... and the asymmetric `expect.any`, `anything`,
 * `stringContaining`, `stringMatching` ... each after the value checked; `.not` before an assertion
 * checks the opposite; `.resolves` or `.rejects` right after `expect(...)`, awaited, checks what a
 * promise settles with"); test.md § Check a result with expect; `Matchers`, `Matching`,
 * `Expectation`, claude-code/index.d.ts:21301–21530 of 2.1.330.
 */
import { expect as bunExpect, test } from 'bun:test';
import { ExpectationError, expect } from '../../../src/testing/mods/index.ts';
import { spec } from '../../support/spec.ts';

/** The message of the `ExpectationError` `check` throws, or `undefined` when it passes. */
function failure(check: () => void): string | undefined {
  try {
    check();
    return undefined;
  } catch (error) {
    bunExpect(error).toBeInstanceOf(ExpectationError);
    return error instanceof Error ? error.message : String(error);
  }
}

async function failureOf(check: () => Promise<void>): Promise<string | undefined> {
  try {
    await check();
    return undefined;
  } catch (error) {
    bunExpect(error).toBeInstanceOf(ExpectationError);
    return error instanceof Error ? error.message : String(error);
  }
}

const boom = () => {
  throw new TypeError('the boom');
};

const passes = (check: () => void) => bunExpect(failure(check)).toBeUndefined();
const fails = (check: () => void) => bunExpect(failure(check)).toBeString();

spec('mods.test.Check a result with expect', () => {
  test('equality: toBe, toEqual ignoring undefined properties, toStrictEqual counting them', () => {
    passes(() => expect(1).toBe(1));
    fails(() => expect({}).toBe({}));
    passes(() => expect({ a: 1, b: undefined }).toEqual({ a: 1 }));
    fails(() => expect({ a: 1, b: undefined }).toStrictEqual({ a: 1 }));
    passes(() => expect({ a: [1, { b: 2 }] }).toStrictEqual({ a: [1, { b: 2 }] }));
    class Point {
      readonly x: number;
      constructor(x: number) {
        this.x = x;
      }
    }
    fails(() => expect(new Point(1)).toStrictEqual({ x: 1 }));
  });

  test('shape: toMatchObject, toContain, toContainEqual, toHaveLength, toHaveProperty', () => {
    passes(() => expect({ a: 1, b: { c: 2, d: 3 } }).toMatchObject({ b: { c: 2 } }));
    passes(() => expect('hello').toContain('ell'));
    passes(() => expect([1, 2]).toContain(2));
    fails(() => expect([{ a: 1 }]).toContain({ a: 1 }));
    passes(() => expect([{ a: 1 }]).toContainEqual({ a: 1 }));
    passes(() => expect(new Set([3])).toContain(3));
    passes(() => expect([1, 2, 3]).toHaveLength(3));
    passes(() => expect('ab').toHaveLength(2));
    passes(() => expect({ a: { b: 2 } }).toHaveProperty('a.b'));
    passes(() => expect({ a: { b: 2 } }).toHaveProperty(['a', 'b'], 2));
    fails(() => expect({ a: { b: 2 } }).toHaveProperty('a.b', 3));
    fails(() => expect({ a: {} }).toHaveProperty('a.c'));
  });

  test('values: toBeUndefined, toBeDefined, toBeNull, toBeTruthy, toBeFalsy, toBeNaN', () => {
    passes(() => expect(undefined).toBeUndefined());
    passes(() => expect(0).toBeDefined());
    passes(() => expect(null).toBeNull());
    passes(() => expect('x').toBeTruthy());
    passes(() => expect('').toBeFalsy());
    passes(() => expect(Number.NaN).toBeNaN());
    fails(() => expect(1).toBeNaN());
  });

  test('order: toBeGreaterThan, toBeGreaterThanOrEqual, toBeLessThan, toBeLessThanOrEqual', () => {
    passes(() => expect(2).toBeGreaterThan(1));
    passes(() => expect(2).toBeGreaterThanOrEqual(2));
    passes(() => expect(1n).toBeLessThan(2n));
    passes(() => expect(2).toBeLessThanOrEqual(2));
    fails(() => expect(2).toBeLessThan(2));
    fails(() => expect('2').toBeGreaterThan(1));
  });

  test('text: toMatch, toStartWith, toEndWith', () => {
    passes(() => expect('hello world').toMatch(/o w/));
    passes(() => expect('hello world').toMatch('lo w'));
    passes(() => expect('hello').toStartWith('he'));
    passes(() => expect('hello').toEndWith('lo'));
    fails(() => expect('hello').toEndWith('he'));
  });

  test('toBeInstanceOf and toThrow by substring, pattern, class or message', () => {
    passes(() => expect(new TypeError('x')).toBeInstanceOf(Error));
    passes(() => expect(boom).toThrow());
    passes(() => expect(boom).toThrow('boom'));
    passes(() => expect(boom).toThrow(/^the/));
    passes(() => expect(boom).toThrow(TypeError));
    passes(() => expect(boom).toThrow({ message: 'the boom' }));
    fails(() => expect(boom).toThrow({ message: 'boom' }));
    fails(() => expect(boom).toThrow(RangeError));
    fails(() => expect(() => undefined).toThrow());
  });

  test('.not before an assertion checks the opposite', () => {
    passes(() => expect(1).not.toBe(2));
    fails(() => expect(1).not.toBe(1));
    passes(() => expect('abc').not.toStartWith('b'));
  });

  test('the asymmetric matchers stand inside an expected value', () => {
    passes(() =>
      expect({ n: 1, s: 'hello', o: {}, l: [1, 2, 3] }).toEqual({
        n: expect.any(Number),
        s: expect.stringContaining('ell'),
        o: expect.anything(),
        l: expect.arrayContaining([3, 1]),
      }),
    );
    passes(() => expect({ s: 'abc' }).toMatchObject({ s: expect.stringMatching(/^a/) }));
    passes(() => expect({ a: 1, b: 2 }).toEqual(expect.objectContaining({ a: 1 })));
    fails(() => expect({ v: null }).toEqual({ v: expect.anything() }));
    fails(() => expect({ v: 'x' }).toEqual({ v: expect.any(Number) }));
  });

  test('.resolves and .rejects check what a promise settles with, and .not after them', async () => {
    bunExpect(await failureOf(() => expect(Promise.resolve(2)).resolves.toBe(2))).toBeUndefined();
    bunExpect(
      await failureOf(() => expect(Promise.resolve(2)).resolves.not.toBe(3)),
    ).toBeUndefined();
    bunExpect(
      await failureOf(() => expect(Promise.reject(new Error('no'))).rejects.toThrow('no')),
    ).toBeUndefined();
    bunExpect(
      await failureOf(() => expect(Promise.reject(new Error('no'))).resolves.toBe(1)),
    ).toBeString();
    bunExpect(await failureOf(() => expect(Promise.resolve(1)).rejects.toThrow())).toBeString();
  });

  test('a message given to expect leads the failure', () => {
    bunExpect(failure(() => expect(1, 'the count').toBe(2))).toStartWith('the count: ');
  });
});
