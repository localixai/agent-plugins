/** What every oracle of `scripts/rust/js-oracle.ts` shares: the edits, the outcome, the line written. */
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const out = join(
  import.meta.dir,
  '..',
  '..',
  '..',
  'crates',
  'agent-plugins',
  'tests',
  'vectors',
  'js',
);

/** Every text one edit away from a seed: each character removed, replaced or preceded by each edit. */
export function edits(seeds: readonly string[], alphabet: readonly string[]): string[] {
  const texts = new Set<string>(seeds);
  for (const seed of seeds) {
    for (let at = 0; at <= seed.length; at += 1) {
      for (const edit of alphabet) {
        texts.add(seed.slice(0, at) + edit + seed.slice(at));
        if (at < seed.length) texts.add(seed.slice(0, at) + edit + seed.slice(at + 1));
      }
    }
  }
  return [...texts].toSorted();
}

/** A value JSON can carry to Rust: no string with a lone surrogate, which no Rust string holds. */
function wellFormed(value: unknown): boolean {
  if (typeof value === 'string') return value.isWellFormed();
  if (Array.isArray(value)) return value.every(wellFormed);
  if (typeof value === 'object' && value !== null) {
    return Object.entries(value).every(([key, item]) => key.isWellFormed() && wellFormed(item));
  }
  return true;
}

export type Outcome = { readonly result: unknown } | { readonly throws: string };

export function outcome(run: () => unknown): Outcome {
  try {
    return { result: run() };
  } catch (error) {
    return { throws: error instanceof Error ? error.message : String(error) };
  }
}

/** JSON's text of an outcome, or undefined when a value in it has none Rust can read back. */
export function line(fn: string, args: readonly unknown[], result: Outcome): string | undefined {
  const value = { fn, args, outcomes: [result], spec: [] };
  return wellFormed(value) ? JSON.stringify(value) : undefined;
}

export function write(name: string, lines: readonly (string | undefined)[]): void {
  mkdirSync(out, { recursive: true });
  const kept = lines.filter((text) => text !== undefined);
  writeFileSync(join(out, `${name}.jsonl`), `${kept.join('\n')}\n`);
  console.log(`${name}: ${kept.length} cases`);
}

/** A number as JSON carries it, or tagged when JSON has no text for it (`String(-0)` is `0`). */
export const numberText = (value: number): unknown => {
  if (Object.is(value, -0)) return { $number: '-0' };
  return Number.isFinite(value) ? value : { $number: String(value) };
};
