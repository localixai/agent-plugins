/**
 * `new RegExp(source, 'u').test(text)`, for the patterns the package takes from input (a
 * schema's `patternProperties`, `js::reg_exp`): whether the source compiles, and if it does what
 * it answers for each of a fixed set of texts.
 */
import { edits, line, outcome, write } from './shared.ts';

/** Patterns of every part of the grammar the core runs, and of what it refuses. */
const SEEDS = [
  '^x$',
  'a|b|',
  '(?:ab)+?c{2,3}',
  '(?<n>a)\\d{1,}\\D',
  '^[a-c\\-_]*$',
  '[^\\s\\S]',
  '\\w+\\W?\\b\\B',
  '\\u{1F600}|\\uD83D\\uDE00',
  '\\x41\\cJ\\0\\t\\/',
  '[\\b\\u00e9-\\u00ff]',
  '.\\.\\$',
  '(?=a)b(?!c)',
  '(a)\\1\\k<n>',
  '\\p{L}',
];
const EDITS = [
  '',
  '\\',
  '(',
  ')',
  '[',
  ']',
  '{',
  '}',
  '-',
  '^',
  '$',
  '|',
  '?',
  '*',
  '+',
  ',',
  ':',
  '<',
  '0',
  'u',
  'x',
  'c',
  'é',
];
/** Sources one edit cannot reach: counts at their limits, surrogates, deep and empty groups. */
const CASES = [
  '[]',
  '[^]',
  '',
  '()',
  'a{0}',
  'a{4294967295}',
  'a{4294967296}',
  'a{3,2}',
  '\\uD800',
  '[\\uD800-\\uDFFF]',
  '[a-\\uD800]',
  '\\u{110000}',
  '\\u{0000000041}',
  '(?<a>x)|(?<a>y)',
  '(?<é>x)',
  '(?i:a)',
  '(?<=a)b',
  '\\c1',
  '[\\c1]',
  '\\8',
  '[\\d-z]',
  '[z-a]',
  '(((((((((((((((((((((a)))))))))))))))))))))',
];
/** The texts each pattern is tested against. */
const TEXTS = ['', 'x', 'ab', 'abcc', 'A\n\t', 'aé-_', '😀', 'ÿ  b', '-0/$.'];

export function recordRegExps(): void {
  const sources = [...edits(SEEDS, EDITS), ...CASES];
  write(
    'reg-exp',
    sources.map((source) =>
      line(
        'RegExp',
        [source, TEXTS],
        outcome(() => {
          const pattern = new RegExp(source, 'u');
          return TEXTS.map((text) => pattern.test(text));
        }),
      ),
    ),
  );
}
