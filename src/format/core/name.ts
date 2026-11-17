/**
 * A core package's `name` (core §5.5): 1–64 characters of `a-z`, `0-9`, `-` and `.`, alphanumeric at
 * both ends, no `--` and no `..`. The checks are the schema's (`schemas/1.1.0/plugin.schema.json`,
 * `name`), each with its own reason so a report says which constraint failed (core §5.3).
 */

const RULES: readonly { readonly fails: (name: string) => boolean; readonly reason: string }[] = [
  {
    fails: (name) => name.length < 1 || name.length > 64,
    reason: 'must be 1 to 64 characters long',
  },
  {
    fails: (name) => /[^a-z0-9.-]/.test(name),
    reason: 'may hold only lowercase letters, digits, "-" and "."',
  },
  {
    fails: (name) => /^[.-]|[.-]$/.test(name),
    reason: 'must start and end with a letter or a digit',
  },
  { fails: (name) => name.includes('--'), reason: 'must not hold "--"' },
  { fails: (name) => name.includes('..'), reason: 'must not hold ".."' },
];

/** Why a name breaks core §5.5, each failed constraint once; empty when it is valid. */
export const coreNameProblems = (name: string): readonly string[] =>
  RULES.filter((rule) => rule.fails(name)).map((rule) => rule.reason);
