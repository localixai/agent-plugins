/**
 * npm's `semver` (the version in package.json), the functions `src/marketplace/semver.ts` calls
 * with their default options: `parse`, `validRange`, `satisfies`, `maxSatisfying`, `rcompare`
 * (and `highestVersion`'s sort by it), `intersects` and `new Range(range).range`. The crate's
 * `semver` module is held to them (`crates/agent-plugins/tests/differential/semver.rs`). Versions
 * and ranges are fixed seeds and every one-character edit of them; each case meets one partner,
 * turning through the others and fixed lists.
 */
import { Range, intersects, maxSatisfying, parse, rcompare, satisfies, validRange } from 'semver';
import { edits, line, outcome, write } from './shared.ts';

const VERSIONS = [
  '1.2.3',
  'v1.2.3',
  '=1.2.3',
  ' 1.2.3 ',
  '1.2.3-beta.1',
  '1.2.3-0',
  '1.2.3+build.5',
  '1.2.3-rc.1+b',
  '01.2.3',
  '1.2.3-01',
  '1.2.3-a.b-c.10',
  '2.1.100',
  '0.0.0',
  '9007199254740991.0.0',
  '9007199254740992.0.0',
  '1.2.3-9007199254740992',
  '1.2',
];
const VERSION_EDITS = ['', '0', '1', '.', '-', '+', 'v', '=', ' ', 'x', 'a'];

const RANGES = [
  '^1.2.3',
  '~1.2',
  '>=1.4',
  '=2.1.0',
  '1.2.3 - 2.3',
  '1.x || >=2.5.0',
  '*',
  '',
  '>1.2.3-beta <2',
  '^0.0.1',
  '^0.1.x',
  '~> 1.2',
  '<=1.2',
  'x.X.*',
  '>=1.0.0 <2.0.0-0',
  '^2.0.0-0',
  '1.2.3+build',
  '>= 1.2.3 < 2',
  '1 - 2',
  '^1.2.3-beta.2',
  '~1.2.3-beta.2',
  '>1',
  '<1.x',
  '>=0.0.0',
  '<0.0.0-0',
  '1.2.3 || ^2',
  '~0.0.1',
  '^ 1.2',
];
const RANGE_EDITS = ['', '0', '.', '-', ' ', 'x', '|', '<', '=', '~', '^'];

/** Ranges one edit cannot reach: long numbers, many alternatives, odd blanks. */
const RANGE_CASES = [
  `^${'9'.repeat(17)}.0.0`,
  `>=${'1'.repeat(300)}`,
  '^9007199254740991.0.0',
  '~9007199254740991',
  ' 1.2.3\t- 1.3 ',
  '1.2.3  ||  2.x ||',
  '||',
  '>=1.2.3 >=1.2.3 <1.2.3',
  '1.2.3-beta+meta - 2.0.0+meta',
  '>=1.2.3-pre.1 <1.2.3-pre.5',
  '<1.2.3 >1.2.3',
  `${'1.2.3 || '.repeat(20)}1.2.3`,
];
const MORE_VERSIONS = [
  '1.2.4',
  '1.3.0',
  '2.0.0',
  '2.0.0-beta.1',
  '2.5.0',
  '0.0.1',
  '0.0.2',
  '0.1.5',
  '1.2.3-beta.3',
  '1.2.4-beta.1',
  '3.0.0',
];

/** What each version is also compared with, and each range intersected with, in turn. */
const PARTNERS = ['1.2.3', '1.2.3-beta.10', '1.2.3-beta.9', '2.0.0'];
const OVERLAPS = ['^1.2.0', '<1.0.0', '>=2.0.0-0', '1.2.3 - 1.2.5', '~1.2.3 || >3'];

const versions = edits(VERSIONS, VERSION_EDITS);
const ranges = [...edits(RANGES, RANGE_EDITS), ...RANGE_CASES];
const probes = [...VERSIONS, ...MORE_VERSIONS];

const parsed = (text: string): unknown => {
  const found = parse(text);
  return found === null
    ? null
    : { version: found.version, raw: found.raw, prerelease: found.prerelease, build: found.build };
};

/** The `index`th of `list`, turning: every case meets a different partner. */
const nth = (list: readonly string[], index: number): string => list[index % list.length] ?? '';

export function recordSemver(): void {
  write('semver', [
    ...versions.map((text) =>
      line(
        'parse',
        [text],
        outcome(() => parsed(text)),
      ),
    ),
    ...ranges.map((range) =>
      line(
        'range',
        [range],
        outcome(() => new Range(range).range),
      ),
    ),
    ...ranges
      .filter((_, index) => index % 8 === 0)
      .map((range) =>
        line(
          'validRange',
          [range],
          outcome(() => validRange(range)),
        ),
      ),
    ...ranges.map((range, index) => {
      const version = nth(probes, index * 5 + 3);
      return line(
        'satisfies',
        [version, range],
        outcome(() => satisfies(version, range)),
      );
    }),
    ...ranges
      .filter((_, index) => index % 8 === 0)
      .map((range) =>
        line(
          'maxSatisfying',
          [probes, range],
          outcome(() => maxSatisfying(probes, range)),
        ),
      ),
    ...versions.map((a, index) => {
      const b = index % 2 === 0 ? nth(versions, index * 7) : nth(PARTNERS, index);
      return line(
        'rcompare',
        [a, b],
        outcome(() => rcompare(a, b)),
      );
    }),
    ...ranges.map((a, index) => {
      const b = index % 2 === 0 ? nth(ranges, index * 13) : nth(OVERLAPS, index);
      return line(
        'intersects',
        [a, b],
        outcome(() => intersects(a, b)),
      );
    }),
    ...[
      [...MORE_VERSIONS],
      ['1.0.0', 'v1.0.0', '1.0.0+b', '1.0.0'],
      ['2.0.0', 'nope', '1.0.0'],
      ['nope', 'also-not'],
      ['1.0.0', '2.0.0', 'bad'],
      ['bad'],
      ['1.0.0-beta', '1.0.0', '0.9.0', '1.0.0+b'],
      [],
    ].map((list) =>
      // `highestVersion`'s own expression; `null` for none, as JSON carries no `undefined`.
      line(
        'highestVersion',
        [list],
        outcome(() => list.toSorted(rcompare)[0] ?? null),
      ),
    ),
  ]);
}
