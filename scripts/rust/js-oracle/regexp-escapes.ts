/**
 * The patterns `regexp.ts` compiles with and without `u`: patterns that do not compile, and
 * every printable ASCII character and a few past ASCII escaped, alone and in a class; escapes whose digits are cut short, signed or out
 * of range; quantified assertions; and patterns that run, whose escapes the engine would read
 * otherwise (`\x+1` is `x+1` without `u`, a lead surrogate's `\uHHHH` before `\u{…}`).
 */
import type { PatternCase } from './regexp-cases.ts';

/** Characters past ASCII: Latin, the long s, the Kelvin sign, joiners, a line separator, an emoji. */
const PAST_ASCII = [
  'é',
  'ÿ',
  'ſ',
  ...[0x212a, 0x200c, 0x200d, 0x2028].map((code) => String.fromCodePoint(code)),
  '😀',
];

const PRINTABLE = Array.from({ length: 0x5f }, (_, offset) => String.fromCharCode(0x20 + offset));

/** Escapes with digits or a tail that may be wrong. */
const TAILS = [
  '\\u',
  '\\u1',
  '\\u12',
  '\\u123',
  '\\u+00e',
  '\\u$',
  '\\u{',
  '\\u{}',
  '\\u{1',
  '\\u{+41}',
  '\\u{-1}',
  '\\u{110000}',
  '\\u{10FFFF}',
  '\\u{0000000041}',
  '\\x',
  '\\x1',
  '\\x+1',
  '\\c',
  '\\c$',
  '\\c1',
  '\\ca',
  '\\00',
  '\\01',
  '\\8',
  '\\1',
  '\\k',
  '\\k<a>',
];

/** Assertions with a quantifier after them. */
const QUANTIFIED = [
  '\\b*',
  '\\B+',
  '\\b{2}',
  '\\B?',
  '[\\b]*',
  '^*',
  '$+',
  '(?=a)*',
  '(?!a)+',
  '(?<=a)*',
  '(?<!a)?',
];

/**
 * Patterns that do not compile, tried with and without `u`. With `v` the engine accepts escapes
 * JavaScriptCore refuses (`\\q`, `\\00`, a lone `]`; docs/RUST.md), and no module compiles with
 * `v`, so `v` is tried with the patterns that compile only.
 */
export const INVALID = [
  '(',
  ')',
  'a)',
  '((a)',
  '[',
  '[a',
  '[b-a]',
  'a{2,1}',
  '*',
  '+a',
  '?',
  'a**',
  'a{1}{2}',
  '\\',
  'a\\',
  '(?<a>x)(?<a>y)',
  '(?<1a>x)',
  '(?<a',
  '(?<>x)',
  '\\k<b>(?<a>x)',
  '(?<a>x)\\k<b>',
  '(?<a>x)\\k',
  '(?',
  '(?x)',
  '(?<=a',
  '(?=a',
  '\\p{Foo}',
  '\\p{L',
  '\\P',
  '\\u{110000}',
  '\\u{',
  '\\c1',
  '\\q',
  '\\1',
  '\\00',
  '[\\d-z]',
  'a{',
  '{',
  ']',
  '}',
  '(?i-i:a)',
  '(?ii:a)',
  'x{99999999999}',
  '[[a-z]--b]',
  '\\u12',
  '^*',
  'a|*',
];

/** Sources tried with and without `u`; the ones that compile and the ones that throw. */
export const ESCAPES: readonly string[] = [
  ...[...PRINTABLE, ...PAST_ASCII].flatMap((character) => [`\\${character}`, `[\\${character}]`]),
  ...TAILS.flatMap((tail) => [tail, `[${tail}]`]),
  ...QUANTIFIED,
];

/** Patterns that run, with the texts that tell one reading of their escapes from another. */
export const ESCAPE_PATTERNS: readonly PatternCase[] = [
  { source: '\\x+1|\\u+00e9', flags: [''], texts: ['xx1', 'uu00e9', '\u00011', '\u000e9'] },
  { source: '^a{+1}$|^b{1,+2}$|^c{ 1}$', flags: [''], texts: ['a{+1}', 'a', 'b{1,+2}', 'c{ 1}'] },
  { source: '[\\x+1]+|\\c+|\\u{+41}', flags: [''], texts: ['x+1', '\\cc', 'u{+41}', 'A'] },
  { source: '\\é|[\\ÿ-\\ſ]', flags: ['u', 'iu'], texts: ['é', 'ſ', 'Ā', 'É'] },
  { source: '\\uD83D\\u{E9}|\\uD83D\\u{DE00}|x\\u{1F600}', flags: ['u'], texts: ['é', 'x😀'] },
];
