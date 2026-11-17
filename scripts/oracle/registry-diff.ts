/**
 * Compares the kit's registry of a fixture with the one the spec recorded, field by field. A field
 * may differ on purpose: tests/fixtures/runtime-judgments.json names the value the kit gives and the
 * row of spec/judgments.md that explains it; anything else that differs is a failure.
 */
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { z } from 'zod';
import { ROOT } from '../docs/location.ts';
import type { Registry } from './recorded.ts';

const JUDGMENTS_FILE = join(ROOT, 'tests', 'fixtures', 'runtime-judgments.json');

export const FIELDS = [
  'loaded',
  'agents',
  'slashCommands',
  'skills',
  'mcpServers',
  'outputStyle',
] as const satisfies readonly (keyof Registry)[];
export type Field = (typeof FIELDS)[number];

const judgmentsSchema = z.record(
  z.string(),
  z.record(
    z.string(),
    z.object({ kit: z.union([z.boolean(), z.string(), z.array(z.string())]), row: z.string() }),
  ),
);
export type Judgments = z.infer<typeof judgmentsSchema>;

export interface FieldVerdict {
  readonly field: Field;
  readonly status: 'agrees' | 'judged' | 'differs';
  readonly expected: Registry[Field];
  readonly kit: Registry[Field];
  /** The row of spec/judgments.md when the difference is a judgment. */
  readonly row?: string;
}

export async function readJudgments(): Promise<Judgments> {
  try {
    return judgmentsSchema.parse(JSON.parse(await readFile(JUDGMENTS_FILE, 'utf8')));
  } catch {
    return {};
  }
}

/** One verdict per field of the registry. */
export function compareRegistries(
  name: string,
  kit: Registry,
  expected: Registry,
  judgments: Judgments,
): FieldVerdict[] {
  return FIELDS.map((field) => {
    const same = JSON.stringify(kit[field]) === JSON.stringify(expected[field]);
    const judged = judgments[name]?.[field];
    const base = { field, expected: expected[field], kit: kit[field] };
    if (same) return { ...base, status: 'agrees' };
    if (judged !== undefined && JSON.stringify(judged.kit) === JSON.stringify(kit[field]))
      return { ...base, status: 'judged', row: judged.row };
    return { ...base, status: 'differs' };
  });
}
