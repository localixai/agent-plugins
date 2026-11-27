/**
 * Case-insensitive matching over every cased BMP character: each one as a literal, a class and a
 * negated class, with `i` and with `iu`, against every character its case is related to (by
 * `toUpperCase` and `toLowerCase`, both ways). Characters newer than JavaScriptCore's case table
 * for patterns are left out (docs/RUST.md).
 */
import { line, outcome } from './shared.ts';

/**
 * Letters Unicode 16 added and the older letters they pair with (U+019B, U+0264), which
 * JavaScriptCore's table for `i` without `u` does not relate though its `toUpperCase` does.
 */
const NEWER = /[ƛɤᲉᲊꟋ-Ƛ]/;

const hex = (unit: number): string => unit.toString(16).padStart(4, '0');

/** The characters each cased one is related to, closed under the relation. */
function caseGroups(): number[][] {
  const related = new Map<number, Set<number>>();
  const relate = (one: number, other: number): void => {
    if (one === other) return;
    for (const [from, to] of [
      [one, other],
      [other, one],
    ] as const) {
      const set = related.get(from) ?? new Set<number>();
      set.add(to);
      related.set(from, set);
    }
  };
  for (let unit = 0; unit < 0x10000; unit += 1) {
    if (unit >= 0xd800 && unit < 0xe000) continue;
    const text = String.fromCharCode(unit);
    for (const mapped of [text.toUpperCase(), text.toLowerCase()]) {
      if (mapped.length === 1) relate(unit, mapped.charCodeAt(0));
    }
  }
  const seen = new Set<number>();
  const groups: number[][] = [];
  for (const start of related.keys()) {
    if (seen.has(start)) continue;
    const group: number[] = [];
    const stack = [start];
    for (let unit = stack.pop(); unit !== undefined; unit = stack.pop()) {
      if (seen.has(unit)) continue;
      seen.add(unit);
      group.push(unit);
      stack.push(...(related.get(unit) ?? []));
    }
    groups.push(group.toSorted((first, second) => first - second));
  }
  return groups;
}

const SHAPES = [
  (unit: number) => `^\\u${hex(unit)}$`,
  (unit: number) => `^[\\u${hex(unit)}]$`,
  (unit: number) => `^[^\\u${hex(unit)}]$`,
];

export function foldLines(): (string | undefined)[] {
  const groups = caseGroups().filter(
    (group) => !group.some((unit) => NEWER.test(String.fromCharCode(unit))),
  );
  return groups.flatMap((group) =>
    group.flatMap((unit) =>
      SHAPES.flatMap((shape) =>
        ['i', 'iu'].flatMap((flags) =>
          group.map((other) => {
            const text = String.fromCharCode(other);
            const source = shape(unit);
            return line(
              'RegExp.index',
              [source, flags, text, 0],
              outcome(() => ({
                found: new RegExp(source, flags).exec(text)?.index ?? -1,
                lastIndex: 0,
              })),
            );
          }),
        ),
      ),
    ),
  );
}
