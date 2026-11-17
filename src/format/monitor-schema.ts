import { z } from 'zod';
import { closedObject } from './strict-keys.ts';

/**
 * A monitor entry and the monitors source as Claude Code's schema check reads them, one rule set
 * for the loader and the validator (plugins/manifest-reference.md, "monitors": "Each entry is a
 * strict object"; spec §9.2.1–§9.2.2). The messages are the oracle's (2.1.325): zod's wording for a
 * missing or empty field, and for `when` and the names the texts the spec quotes. A missing field
 * aborts the entry, so inside the source union it is a bare `Invalid input` at the key.
 */

export const ALWAYS = 'always';
const ON_SKILL_INVOKE = 'on-skill-invoke:';

const WHEN_PREFIX = `Invalid string: must start with "${ON_SKILL_INVOKE}"`;
export const WHEN_SKILL = 'on-skill-invoke: must specify a skill name';
export const UNIQUE_NAMES = 'Monitor names must be unique within a plugin';

const text = z.string().min(1);

/** The skill after the prefix, empty when there is none or no prefix (spec J9.8: read as written). */
export const skillOf = (when: string): string =>
  when.startsWith(ON_SKILL_INVOKE) ? when.slice(ON_SKILL_INVOKE.length) : '';

/**
 * `"always"` or `"on-skill-invoke:<skill>"` with a non-empty skill. A value that is neither breaks
 * both rules and is reported twice, as the oracle does.
 */
const when = z
  .string()
  .refine((value) => value === ALWAYS || value.startsWith(ON_SKILL_INVOKE), { error: WHEN_PREFIX })
  .refine((value) => value === ALWAYS || skillOf(value).length > 0, { error: WHEN_SKILL });

export const monitorEntrySchema = closedObject({
  name: text,
  command: text,
  description: text,
  when: when.exactOptional(),
});

/** Two entries of one plugin never share a name; reported once, at the array. */
const monitorListSchema = z
  .array(monitorEntrySchema)
  .refine((entries) => new Set(entries.map((entry) => entry.name)).size === entries.length, {
    error: UNIQUE_NAMES,
  });

/**
 * `experimental.monitors` (and the top-level `monitors` that still loads): a path or the inline
 * array. A path is any string here; its `./` and `.json` rules are the path checks'.
 */
export const monitorSourcesSchema = z.union([z.string(), monitorListSchema]);
