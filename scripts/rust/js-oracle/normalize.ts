/** `String.prototype.normalize(form)`, `toUpperCase` and `toLowerCase`: what folds a name. */
import { line, outcome, write } from './shared.ts';

const FORMS = ['NFC', 'NFD', 'NFKC', 'NFKD'] as const;

/** A text of the code points given. */
const of = (...points: readonly number[]): string => String.fromCodePoint(...points);

/** Every `step`-th code point from `from` to `to`, each a text of its own. */
function sweep(from: number, to: number, step: number): string[] {
  const texts: string[] = [];
  for (let point = from; point <= to; point += step) texts.push(of(point));
  return texts;
}

/**
 * Hangul: precomposed syllables across the block, and conjoining jamo that compose (L V, L V T,
 * an LV syllable then a T) or do not (a T alone, V before L, an old jamo).
 */
const HANGUL = [
  ...sweep(0xac00, 0xd7a3, 97),
  of(0x1100, 0x1161),
  of(0x1100, 0x1161, 0x11a8),
  of(0xac00, 0x11a8),
  of(0xac01, 0x11a8),
  of(0x11a8),
  of(0x1161, 0x1100),
  of(0x1100, 0x1100, 0x1161, 0x1161),
  of(0x1112, 0x1175, 0x11c2),
  of(0x115f, 0x1160),
  of(0x1113, 0x1161),
  of(0x1100, 0x1176),
  of(0x3131, 0x314f),
];

/** Combining marks of different classes (1, 7, 10, 202, 220, 230, 240) and the bases they meet. */
const MARKS = [0x334, 0x93c, 0x5b0, 0x328, 0x323, 0x316, 0x301, 0x300, 0x308, 0x345, 0x30a];
const BASES = [0x41, 0x61, 0x65, 0x6f, 0x75, 0x3b1, 0x3c9, 0x415, 0x928, 0x5d0, 0x1ea1];

/** Every base with every ordered pair of marks, and with three in a fixed odd order. */
function marked(): string[] {
  const texts: string[] = [];
  for (const base of BASES) {
    for (const first of MARKS) {
      for (const second of MARKS) texts.push(of(base, first, second));
    }
    texts.push(of(base, 0x301, 0x334, 0x323), of(base, 0x345, 0x308, 0x301));
  }
  return texts;
}

/**
 * Compatibility characters (ligatures, widths, circled, squared, fractions, super- and
 * subscripts, letterlike, Arabic presentation forms), singletons and composition exclusions,
 * astral characters (musical symbols, mathematical letters, Kaithi, emoji, CJK compatibility
 * ideographs), the Greek and Latin letters whose case maps to more than one, and the final sigma.
 */
const FIXED = [
  ...sweep(0xfb00, 0xfb06, 1),
  ...sweep(0xff01, 0xff5e, 7),
  ...sweep(0x2460, 0x24ff, 5),
  ...sweep(0x3300, 0x33ff, 9),
  ...sweep(0x2150, 0x218b, 1),
  ...sweep(0x2070, 0x209c, 1),
  ...sweep(0x2100, 0x214f, 1),
  ...sweep(0xfdf0, 0xfdfd, 1),
  ...sweep(0xfe70, 0xfefc, 11),
  of(0x212b),
  of(0x2126),
  of(0x212a),
  of(0x1e9b, 0x323),
  of(0x958),
  of(0x2adc),
  of(0xf73),
  of(0xf77),
  of(0x344),
  of(0x340, 0x341),
  of(0x1d15e),
  of(0x1d15f, 0x1d165),
  of(0x1d1bb),
  ...sweep(0x1d400, 0x1d7ff, 37),
  of(0x11099, 0x110ba),
  of(0x110ab),
  of(0x11131, 0x11127),
  of(0x1f600),
  of(0x1f1e6, 0x1f1e8),
  ...sweep(0x2f800, 0x2fa1d, 23),
  ...sweep(0xf900, 0xfad9, 31),
  of(0xdf),
  of(0x130),
  of(0x131),
  of(0x149),
  of(0x1f0),
  of(0x390),
  of(0x587),
  of(0x1e96),
  of(0x1f80),
  of(0x1fb3),
  of(0x1fbc),
  of(0xfb13),
  'Σ',
  'ΑΣ',
  'ΑΣ Β',
  'ΑΣ.',
  'Α.Σ',
  'ΑΣ́',
  'İstanbul',
  'ǅ',
  'ﬃ',
  '',
  'plain ascii',
  'Ab. ',
  'Café.txt',
  'Café.TXT',
];

/** A fixed stream of numbers, so the mixed texts are the same on every run. */
function* numbers(seed: number): Generator<number> {
  let state = seed;
  for (;;) {
    state = (state * 1_103_515_245 + 12_345) % 2_147_483_648;
    yield state;
  }
}

/** Texts of two to six characters drawn from all of the above, so neighbours meet in any order. */
function mixed(pool: readonly string[], count: number): string[] {
  const characters = [...new Set(pool.flatMap((text) => Array.from(text)))].toSorted();
  const stream = numbers(20_261_126);
  const draw = (): number => stream.next().value ?? 0;
  const texts: string[] = [];
  for (let made = 0; made < count; made += 1) {
    const length = 2 + (draw() % 5);
    let text = '';
    for (let at = 0; at < length; at += 1) text += characters[draw() % characters.length] ?? '';
    texts.push(text);
  }
  return texts;
}

const MIXED = 1500;

export const recordNormalize = (): void => {
  const fixed = [...HANGUL, ...marked(), ...FIXED];
  const texts = [...new Set([...fixed, ...mixed(fixed, MIXED)])];
  write(
    'normalize',
    texts.flatMap((text) =>
      FORMS.map((form) =>
        line(
          'normalize',
          [text, form],
          outcome(() => text.normalize(form)),
        ),
      ),
    ),
  );
  write(
    'case',
    texts.flatMap((text) => [
      line(
        'toUpperCase',
        [text],
        outcome(() => text.toUpperCase()),
      ),
      line(
        'toLowerCase',
        [text],
        outcome(() => text.toLowerCase()),
      ),
    ]),
  );
};
