/**
 * npm's `ignore` (the version in package.json), as the path rules called it before the core matched
 * gitignore patterns itself: `ignore({ ignorecase }).add(lines).ignores(path)`, both `ignorecase`
 * values. The crate's `ignore` module is held to it (`crates/agent-plugins/tests/differential/
 * ignore.rs`). Patterns are fixed seeds and every one-character edit of them, each tested against
 * paths made from it (its own text, below a directory, above a file, as a directory probed through
 * a name below it) and a turning list of fixed paths and their edits; lists of several lines cover
 * negation and its order. `ignoresLong` records a pattern of one character repeated, the length
 * JavaScriptCore's `RegExp` refuses ("regular expression too large") lying between its two counts.
 */
import ignore from 'ignore';
import { edits, line, outcome, write } from './shared.ts';

const PATTERNS = [
  'a',
  'a.js',
  '*.js',
  'a*',
  '*',
  '**',
  '***',
  '/**',
  '**/a',
  'a/**',
  'a/**/b',
  'a/***/b',
  '**/a/b',
  'a/**/',
  '/a',
  'a/',
  '/a/',
  'a/b',
  'a/*',
  'a/*/b',
  'f*o/*/*',
  '*a*b*',
  '?',
  'a?c',
  '[a-z]',
  '[!a]b',
  '[^a]',
  '[]a]',
  '[a-]',
  '[z-a]',
  '[[:alpha:]]x',
  '[[:nope:]]',
  '[[:digit:]-]',
  '[a',
  '\\*',
  '\\[a]',
  '\\d',
  '\\?',
  'a\\',
  '\\\\',
  'a\\ ',
  'a\\\\ ',
  'a  ',
  '#a',
  '\\#a',
  '!a',
  '\\!a',
  '!',
  '/',
  'foo.bar',
  'a.b.c/d.e',
  '.a',
  'a b',
  '﻿a',
  'é/*',
  'x',
  '',
];
const PATTERN_EDITS = ['', 'a', '*', '/', '!', '\\', ' ', '?', '[', ']', '.', '#', '-', '^'];

const PATHS = [
  'a',
  'a/b',
  'a/b/c',
  'b/a',
  'x/a/b',
  '.a',
  'a.js',
  'x/a.js',
  'A',
  'A/B.JS',
  'foo.bar',
  'a.b.c/d.e',
  'a/',
  'a/b/',
  '*',
  '#a',
  '!a',
  'a b',
  'a ',
  'é/x',
  'a/.claude-kit-directory-probe',
  'x/y/.claude-kit-directory-probe',
  '',
  '/a',
  './a',
  '../a',
  '.',
  '..',
  '..a',
  'a//b',
  '[a]',
];
const PATH_EDITS = ['', 'a', '/', '.', 'b', 'A'];

/** Lists whose lines meet each other: negation, its order, and a parent excluded before a child. */
const LISTS = [
  ['a', '!a'],
  ['!a', 'a'],
  ['*.js', '!b.js'],
  ['a/', '!a/b'],
  ['a/*', '!a/b'],
  ['/a/*', '!/a/b'],
  ['**/node_modules', '!node_modules/x'],
  ['a/**', '!a/b/**', 'a/b/c'],
  ['#c', '', 'a', '\\#c'],
  ['src/**', '!src/*.ts'],
  ['*', '!*/', '!*.md'],
  ['a\r', 'b\n'],
  ['a\nb'],
  ['[a', 'b'],
];
const LIST_PATHS = [
  'a',
  'a/b',
  'a/b/c',
  'b.js',
  'a.js',
  'node_modules/x',
  'src/a.ts',
  'x/y.md',
  'x',
  '#c',
  'b',
];

const patterns = edits(PATTERNS, PATTERN_EDITS);
const paths = edits(PATHS, PATH_EDITS);

/** The `index`th of `list`, turning: every case meets a different partner. */
const nth = (list: readonly string[], index: number): string => list[index % list.length] ?? '';

/** A path a pattern names: its wildcards and escapes made plain, its anchors and negation left out. */
const named = (pattern: string): string =>
  pattern
    .replace(/^[!\\]+/, '')
    .replace(/\\/g, '')
    .replace(/\*+/g, 'x')
    .replace(/\?/g, 'y')
    .replace(/^\/+/, '')
    .replace(/\/+$/, '');

/** The paths one pattern is tried at. */
function pathsFor(pattern: string, index: number): string[] {
  const own = named(pattern);
  return [
    own,
    `d/${own}`,
    `${own}/f`,
    `${own}/.claude-kit-directory-probe`,
    nth(paths, index * 7),
    nth(paths, index * 7 + 3),
  ];
}

const ignores = (lines: readonly string[], ignorecase: boolean, path: string): unknown =>
  ignore({ ignorecase })
    .add([...lines])
    .ignores(path);

/** One case of `ignores`. */
const one = (lines: readonly string[], ignorecase: boolean, path: string): string | undefined =>
  line(
    'ignores',
    [lines, ignorecase, path],
    outcome(() => ignores(lines, ignorecase, path)),
  );

/** The longest pattern of one character `ignore` compiles: its source is the text and 17 more. */
const LONGEST = 1024 * 1024 - 17;

export function recordIgnore(): void {
  write('ignore', [
    ...patterns.flatMap((pattern, index) =>
      pathsFor(pattern, index).map((path, at) => one([pattern], (index + at) % 2 === 1, path)),
    ),
    ...paths.flatMap((path, index) => [
      one([nth(patterns, index * 11)], false, path),
      one([nth(patterns, index * 11 + 5)], true, path),
    ]),
    ...LISTS.flatMap((lines) =>
      LIST_PATHS.flatMap((path) => [one(lines, false, path), one(lines, true, path)]),
    ),
    ...[LONGEST, LONGEST + 1].flatMap((count) =>
      [false, true].map((ignorecase) =>
        line(
          'ignoresLong',
          ['a', count, ignorecase, 'probe'],
          outcome(() => ignores(['a'.repeat(count)], ignorecase, 'probe')),
        ),
      ),
    ),
  ]);
}
