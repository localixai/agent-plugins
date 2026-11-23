import {
  asymmetric,
  equal,
  isObjectOrArray,
  matchesObject,
  type Asymmetric,
} from './expect-equal.ts';
import { CHECKS, errorMatches, show, type Check, type CheckName } from './expect-matchers.ts';

/**
 * `expect` of the test kit (`Expecting`, `Expectation`, `Matchers`, `Matching`,
 * claude-code/index.d.ts:21301–21530 of 2.1.330; Agent Plugins Extended §10.12): each check after the
 * value checked, `.not` before a check for the opposite, `.resolves` and `.rejects` for what a
 * promise settles with, and the asymmetric matchers inside an expected value. A failed check throws
 * an `ExpectationError`, the test's own message leading.
 */

export class ExpectationError extends Error {
  override readonly name = 'ExpectationError';
}

type Checks = { readonly [N in CheckName]: (...args: readonly unknown[]) => void };
type AsyncChecks = { readonly [N in CheckName]: (...args: readonly unknown[]) => Promise<void> };
type Negatable<T> = T & { readonly not: T };

export type Expectation = Negatable<Checks> & {
  readonly resolves: Negatable<AsyncChecks>;
  readonly rejects: Negatable<AsyncChecks>;
};

/** The asymmetric matchers (`Matching`). */
export interface Matching {
  any(expected: abstract new (...args: never[]) => unknown): Asymmetric;
  anything(): Asymmetric;
  stringContaining(text: string): Asymmetric;
  stringMatching(pattern: string | RegExp): Asymmetric;
  objectContaining(shape: object): Asymmetric;
  arrayContaining(items: readonly unknown[]): Asymmetric;
}

export type Expect = ((actual: unknown, message?: string) => Expectation) & Matching;

/** Where the checked value came from, as a failure words it. */
interface Subject {
  readonly lead: string;
  /** Under `.rejects`: `toThrow` describes the rejection itself. */
  readonly isRejection: boolean;
}

const holdsFor = (name: CheckName, check: Check, subject: Subject) =>
  subject.isRejection && name === 'toThrow'
    ? (actual: unknown, args: readonly unknown[]) => errorMatches(actual, args[0])
    : check.holds;

function checkOf(name: CheckName, subject: Subject, negated: boolean) {
  const check: Check = CHECKS[name];
  const holds = holdsFor(name, check, subject);
  return (actual: unknown, args: readonly unknown[]): void => {
    if (holds(actual, args) !== negated) return;
    const not = negated ? 'not ' : '';
    throw new ExpectationError(
      `${subject.lead}expected ${show(actual)} ${not}to ${check.says(args)}`,
    );
  };
}

function checks(actual: unknown, subject: Subject, negated: boolean): Checks {
  const names = Object.keys(CHECKS).filter((name): name is CheckName => name in CHECKS);
  const entries = names.map((name) => {
    const run = checkOf(name, subject, negated);
    return [name, (...args: readonly unknown[]) => run(actual, args)] as const;
  });
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- one entry for each check of the table, which is what `Checks` is
  return Object.fromEntries(entries) as unknown as Checks;
}

type Settled = { readonly value: unknown } | { readonly reason: unknown };

function asyncChecks(
  promise: unknown,
  lead: string,
  rejects: boolean,
  negated: boolean,
): AsyncChecks {
  const settled: Promise<Settled> = Promise.resolve(promise).then(
    (value) => ({ value }),
    (reason: unknown) => ({ reason }),
  );
  const subject: Subject = { lead, isRejection: rejects };
  const names = Object.keys(CHECKS).filter((name): name is CheckName => name in CHECKS);
  const entries = names.map((name) => {
    const run = checkOf(name, subject, negated);
    const check = async (...args: readonly unknown[]): Promise<void> => {
      const outcome = await settled;
      if (rejects && 'value' in outcome) {
        throw new ExpectationError(
          `${lead}expected the promise to reject, it resolved with ${show(outcome.value)}`,
        );
      }
      if (!rejects && 'reason' in outcome) {
        throw new ExpectationError(
          `${lead}expected the promise to resolve, it rejected with ${show(outcome.reason)}`,
        );
      }
      run('value' in outcome ? outcome.value : outcome.reason, args);
    };
    return [name, check] as const;
  });
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- one entry for each check of the table, which is what `AsyncChecks` is
  return Object.fromEntries(entries) as unknown as AsyncChecks;
}

const negatable = <T>(make: (negated: boolean) => T): Negatable<T> => ({
  ...make(false),
  not: make(true),
});

/** A primitive matches `expect.any` by its wrapper: `expect.any(Number)` matches `1`. */
const isAny = (actual: unknown, expected: abstract new (...args: never[]) => unknown): boolean =>
  actual !== null &&
  actual !== undefined &&
  (actual instanceof expected || Object(actual) instanceof expected);

const MATCHING: Matching = {
  any: (expected) =>
    asymmetric(`expect.any(${expected.name})`, (actual) => isAny(actual, expected)),
  anything: () =>
    asymmetric('expect.anything()', (actual) => actual !== null && actual !== undefined),
  stringContaining: (text) =>
    asymmetric(
      `expect.stringContaining(${show(text)})`,
      (actual) => typeof actual === 'string' && actual.includes(text),
    ),
  stringMatching: (pattern) => {
    const regexp = typeof pattern === 'string' ? new RegExp(pattern) : pattern;
    return asymmetric(
      `expect.stringMatching(${String(regexp)})`,
      (actual) => typeof actual === 'string' && regexp.test(actual),
    );
  },
  objectContaining: (shape) =>
    asymmetric(
      `expect.objectContaining(${show(shape)})`,
      (actual) => isObjectOrArray(actual) && matchesObject(actual, shape),
    ),
  arrayContaining: (items) =>
    asymmetric(
      `expect.arrayContaining(${show(items)})`,
      (actual) =>
        Array.isArray(actual) &&
        items.every((item) => actual.some((element) => equal(element, item))),
    ),
};

export const expect: Expect = Object.assign((actual: unknown, message?: string): Expectation => {
  const lead = message === undefined ? '' : `${message}: `;
  return {
    ...negatable((negated) => checks(actual, { lead, isRejection: false }, negated)),
    resolves: negatable((negated) => asyncChecks(actual, lead, false, negated)),
    rejects: negatable((negated) => asyncChecks(actual, lead, true, negated)),
  };
}, MATCHING);
