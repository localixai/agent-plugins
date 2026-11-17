import type { Function as FunctionNode } from 'acorn';
import { isStreamEvent } from '../../chain/wildcard.ts';
import type { ModScanIssueCode } from './issue-codes.ts';

/**
 * Two rules of the `on` calls of `register` (Agent Plugins Extended §10.4.2): `on` is called at most
 * once per event without a matcher, and a hook on a streaming event is an async generator, every
 * other hook an async function. A module that breaks either does not load. The messages are Claude
 * Code 2.1.325's (Judgment J10.2), the first naming the file and line of the first registration.
 */

export interface OnRules {
  readonly file: string;
  /** The 1-based line of an offset in the file's source. */
  readonly lineOf: (offset: number) => number;
  /** The line of the first `on` of each event registered without a matcher. */
  readonly firsts: Map<string, number>;
}

interface OnCall {
  readonly event: string;
  readonly hasMatcher: boolean;
  /** Where the `on` call starts in the source. */
  readonly at: number;
  readonly fn?: FunctionNode;
}

interface OnProblem {
  readonly code: ModScanIssueCode;
  readonly message: string;
}

function twiceProblem(rules: OnRules, call: OnCall): OnProblem | undefined {
  if (call.hasMatcher) return undefined;
  const first = rules.firsts.get(call.event);
  if (first === undefined) {
    rules.firsts.set(call.event, rules.lineOf(call.at));
    return undefined;
  }
  return {
    code: 'on-twice',
    message: `on("${call.event}") is registered twice without a matcher; the first is at ${rules.file}:${String(first)}`,
  };
}

function generatorProblem(call: OnCall): OnProblem | undefined {
  const { fn, event } = call;
  if (fn === undefined || !isStreamEvent(event) || (fn.async && fn.generator)) return undefined;
  return {
    code: 'not-generator',
    message: `the hook on "${event}" is not an async generator: ${event} streams, so it takes async function* ($, e, next) { ... }, which yields the chunks and returns the result`,
  };
}

/** What breaks the two rules for one `on` call, in the order the validator reports them. */
export const onProblems = (rules: OnRules, call: OnCall): OnProblem[] =>
  [twiceProblem(rules, call), generatorProblem(call)].filter(
    (problem): problem is OnProblem => problem !== undefined,
  );

/** The 1-based line of each offset of `source`. */
export function lineIndex(source: string): (offset: number) => number {
  const starts = [0];
  for (let at = source.indexOf('\n'); at !== -1; at = source.indexOf('\n', at + 1)) {
    starts.push(at + 1);
  }
  return (offset) => {
    let line = 0;
    while (line + 1 < starts.length && (starts[line + 1] ?? Infinity) <= offset) line += 1;
    return line + 1;
  };
}
