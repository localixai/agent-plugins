import { z } from 'zod';
import { codePointLength } from './code-points.ts';

/**
 * The `options` rules of a `userConfig` option (plugins/manifest-reference.md, "Limit a field to
 * fixed options": "`options` applies to a `string` field that isn't `multiple` or `sensitive`",
 * each value "a plain label of 1 to 64 characters", "Set `default` to one of the listed values, or
 * set `required: true`"; a plugin that breaks them "fails to load"). The messages, paths, order and
 * the rules the docs leave to "anything else it rejects" are the oracle's (2.1.325, spec §3.4.2 and
 * J3.10): length in code points; no edge space; no control, invisible or bidirectional-formatting
 * character; no repeat in any letter case; at least one value.
 */

const MAX_CODE_POINTS = 64;
/** Controls (`Cc`) and format characters (`Cf`: U+200B and the other invisible and bidi ones). */
const HIDDEN = /[\p{Cc}\p{Cf}]/u;

/** The first rule one value breaks, in the oracle's order; `undefined` when it keeps them all. */
function valueProblem(value: string): string | undefined {
  if (value.length === 0) return 'An option cannot be empty';
  if (codePointLength(value) > MAX_CODE_POINTS) return 'An option can be at most 64 characters';
  if (value.trim() !== value) return 'An option cannot start or end with a space';
  if (HIDDEN.test(value)) {
    return 'An option cannot contain control, invisible or bidirectional-formatting characters';
  }
  return undefined;
}

/** One value of `options`: its problem is reported at `options.<i>`. */
export const optionValue = z.string().check((context) => {
  const problem = valueProblem(context.value);
  if (problem === undefined) return;
  context.issues.push({ code: 'custom', input: context.value, message: problem, continue: true });
});

/** What the list-level rules read of an option. */
interface OptionsHolder {
  readonly type: string;
  readonly options?: readonly string[] | undefined;
  readonly default?: unknown;
  readonly required?: boolean | undefined;
  readonly multiple?: boolean | undefined;
  readonly sensitive?: boolean | undefined;
}

interface Problem {
  readonly path: 'options' | 'default';
  readonly message: string;
}

/** The types `options` applies to: "For `string`, the values the field accepts". */
const TYPES_WITH_OPTIONS: ReadonlySet<string> = new Set(['string']);

const ONLY_FOR_STRING =
  'options is only for a field of type "string" that is neither multiple nor sensitive';

const repeats = (options: readonly string[]): boolean =>
  new Set(options.map((value) => value.toLowerCase())).size !== options.length;

/** The default rule: one of the values, compared exactly, or `required: true` (J3.11). */
function defaultProblem(option: OptionsHolder, options: readonly string[]): Problem | undefined {
  const fallback = option.default;
  if (fallback === undefined) {
    return option.required === true
      ? undefined
      : {
          path: 'default',
          message: 'a field with options needs a default among them, or required: true',
        };
  }
  if (typeof fallback === 'string' && options.includes(fallback)) return undefined;
  return { path: 'default', message: `default must be one of the options: ${options.join(', ')}` };
}

/**
 * The list-level problems, in the oracle's order. A field that may not have `options` at all
 * reports that alone (oracle 2.1.325: a `multiple` field with a wrong `default` draws one error).
 */
export function optionsProblems(option: OptionsHolder): Problem[] {
  const { options } = option;
  if (options === undefined) return [];
  const allowed =
    TYPES_WITH_OPTIONS.has(option.type) && option.multiple !== true && option.sensitive !== true;
  if (!allowed) return [{ path: 'options', message: ONLY_FOR_STRING }];
  const problems: Problem[] = [];
  if (options.length === 0)
    problems.push({ path: 'options', message: 'options needs at least one value' });
  if (repeats(options)) {
    problems.push({
      path: 'options',
      message: 'options cannot repeat a value (in any letter case)',
    });
  }
  const fallback = defaultProblem(option, options);
  if (fallback) problems.push(fallback);
  return problems;
}
