/** `Number(text)`, `String(number)` and `String.prototype.trim`. */
import { edits, line, numberText, write } from './shared.ts';

const NUMBER_SEEDS = [
  '12',
  ' 0x1F ',
  '-1.5e3',
  '.5',
  '1_000',
  'Infinity',
  '0b101',
  '1e400',
  '0o17',
  '+.5E-3',
  '-0',
  '4.9e-324',
  '0x20000000000003',
];
/** Numbers past a double's 53 bits, its range, and its rounding, which one edit cannot reach. */
const NUMBER_CASES = [
  '9007199254740993',
  '1.7976931348623159e308',
  '2.4703282292062328e-324',
  '-1e-400',
  `0x${'f'.repeat(40)}`,
  `0x${'f'.repeat(300)}`,
  `0b${'1'.repeat(60)}`,
  `0o${'7'.repeat(30)}`,
  '0x20000000000001000000001',
  '1'.repeat(400),
  `0.${'0'.repeat(400)}1`,
  '00012.50',
  '\u3000\u2028 7 \ufeff',
  '\u0085 1',
  '1 \u180e',
  '-0x1',
  '+Infinity',
  'infinity',
  'NaN',
];
const NUMBER_EDITS = [
  'E',
  '_',
  'I',
  '\u3000',
  '',
  ' ',
  '\t',
  ' ',
  '﻿',
  '+',
  '-',
  '.',
  'e',
  'x',
  'o',
  '0',
  '9',
];

const NUMBERS = [
  0,
  1,
  -1,
  0.1,
  1 / 3,
  1e21,
  1e-7,
  123456789012345680000,
  5e-324,
  1.7976931348623157e308,
  1e-6,
  1.5e-7,
  2 ** 53,
  2 ** 53 + 2,
  0.1 + 0.2,
  1e100,
  4.35,
  123e-20,
  2 ** -1074 * 3,
  Number.NaN,
];

const BLANKS = [' ', '\t', '\n', '\u000b', '\u000c', '\r', '\u0085', ' ', ' ', '᠎'];
const MORE = [' ', ' ', '​', ' ', ' ', ' ', ' ', '　', '﻿'];

export const recordNumbers = (): void => {
  write(
    'number',
    [...edits(NUMBER_SEEDS, NUMBER_EDITS), ...NUMBER_CASES].map((text) =>
      line('Number', [text], { result: numberText(Number(text)) }),
    ),
  );
  write(
    'number-to-string',
    NUMBERS.flatMap((value) => [value, -value, value * 10, value / 10]).map((value) =>
      line('String', [numberText(value)], { result: String(value) }),
    ),
  );
  write(
    'trim',
    [...BLANKS, ...MORE].flatMap((blank) =>
      [`${blank}a${blank}`, `a${blank}b`, blank].map((text) =>
        line('trim', [text], { result: text.trim() }),
      ),
    ),
  );
};
