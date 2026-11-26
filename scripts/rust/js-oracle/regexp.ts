/**
 * `new RegExp(source, flags)` with what the package's modules call on it: `exec` (the match, its
 * index in UTF-16 code units, the captures, the named groups, `lastIndex` after a global or
 * sticky one), `replace` with a replacement text, `split` and `matchAll`, and the `SyntaxError`
 * a pattern or flags that do not compile throw. A match whose text holds a lone surrogate has no
 * JSON a Rust string reads back, so `RegExp.index` records where every match starts as well.
 */
import {
  COMMON_TEXTS,
  FLAGS,
  PATTERNS,
  REPLACED,
  REPLACEMENTS,
  SPLIT,
  type PatternCase,
} from './regexp-cases.ts';
import { ESCAPES, INVALID } from './regexp-escapes.ts';
import { foldLines } from './regexp-fold.ts';
import { line, outcome, write } from './shared.ts';

/** A match as the oracle records it: `null` stands for `undefined` (no group, no `groups`). */
const matchOf = (found: RegExpExecArray) => ({
  index: found.index,
  captures: [...found].map((capture) => capture ?? null),
  groups: found.groups === undefined ? null : { ...found.groups },
});

/** The `lastIndex` values a global or sticky pattern is tried from: the start, inside, the end. */
const startsFor = (flags: string, text: string): readonly number[] =>
  /[gy]/.test(flags) ? [...new Set([0, 1, 2, text.length, text.length + 1])] : [0];

function execLines(source: string, flags: string, text: string): (string | undefined)[] {
  return startsFor(flags, text).flatMap((start) => {
    const run = (answer: (found: RegExpExecArray | null) => unknown) => () => {
      const regexp = new RegExp(source, flags);
      regexp.lastIndex = start;
      const found = regexp.exec(text);
      return { found: answer(found), lastIndex: regexp.lastIndex };
    };
    const args = [source, flags, text, start];
    return [
      line('RegExp.exec', args, outcome(run((found) => (found === null ? null : matchOf(found))))),
      line('RegExp.index', args, outcome(run((found) => (found === null ? -1 : found.index)))),
    ];
  });
}

const textsOf = (each: PatternCase): readonly string[] => [...COMMON_TEXTS, ...(each.texts ?? [])];

function recordExec(): void {
  const lines = PATTERNS.flatMap((each) =>
    each.flags.flatMap((flags) =>
      textsOf(each).flatMap((text) => execLines(each.source, flags, text)),
    ),
  );
  write('regexp-exec', lines);
}

/** What compiles and what throws: every invalid pattern and escape with and without `u`, every flag set. */
const compile = (source: string, flags: string) =>
  line(
    'RegExp',
    [source, flags],
    outcome(() => new RegExp(source, flags).flags),
  );

function recordSyntax(): void {
  write('regexp-syntax', [
    ...INVALID.flatMap((source) => ['', 'u'].map((flags) => compile(source, flags))),
    ...ESCAPES.flatMap((source) => ['', 'u'].map((flags) => compile(source, flags))),
    ...FLAGS.map((flags) => compile('a', flags)),
    ...PATTERNS.flatMap((each) => each.flags.map((flags) => compile(each.source, flags))),
  ]);
}

function recordMethods(): void {
  const replaced = REPLACED.flatMap((each) =>
    each.flags.flatMap((flags) =>
      textsOf(each).flatMap((text) =>
        REPLACEMENTS.map((replacement) =>
          line(
            'replace',
            [each.source, flags, text, replacement],
            outcome(() => text.replace(new RegExp(each.source, flags), replacement)),
          ),
        ),
      ),
    ),
  );
  const split = [...SPLIT, ...PATTERNS].flatMap((each) =>
    each.flags.flatMap((flags) =>
      textsOf(each).map((text) =>
        line(
          'split',
          [each.source, flags, text],
          outcome(() => text.split(new RegExp(each.source, flags)).map((piece) => piece ?? null)),
        ),
      ),
    ),
  );
  const all = PATTERNS.flatMap((each) =>
    each.flags.flatMap((flags) =>
      textsOf(each).map((text) =>
        line(
          'matchAll',
          [each.source, flags, text],
          outcome(() => [...text.matchAll(new RegExp(each.source, flags))].map(matchOf)),
        ),
      ),
    ),
  );
  write('regexp-replace', replaced);
  write('regexp-split', split);
  write('regexp-match-all', all);
}

export const recordRegExp = (): void => {
  recordExec();
  write('regexp-fold', foldLines());
  recordSyntax();
  recordMethods();
};
