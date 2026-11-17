import { isRecord } from '../../protocol/is-record.ts';
import { codePointLength } from '../code-points.ts';
import { parseFrontmatter } from '../frontmatter.ts';

/**
 * A core package's skill (core §7.1): its `SKILL.md` conforms to the Agent Skills specification
 * (agentskills.io/specification, "Frontmatter"), or the client skips the skill. The frontmatter is
 * required; `name` is 1–64 characters of `a-z`, `0-9` and `-`, no hyphen at either end, no `--`,
 * and the parent directory's name; `description` is 1–1024 characters; `license` is a string,
 * `compatibility` 1–500 characters, `metadata` a map of strings, `allowed-tools` a space-separated
 * string.
 *
 * Judgments (spec/judgments.md): a field the Agent Skills specification does not define is a
 * warning, not a skip, since the specification lists its fields without forbidding others (Agent
 * Plugins Extended §18.3.2 lets them exist); a description of blanks only is empty.
 */

export interface CoreSkillCheck {
  /** Why the skill is skipped; empty when it conforms. */
  readonly problems: readonly string[];
  /** Fields outside the Agent Skills specification, in the order they appear. */
  readonly unknownFields: readonly string[];
}

type Check = (value: unknown, directory: string) => string | undefined;

const lengthBetween = (value: unknown, max: number): boolean =>
  typeof value === 'string' && codePointLength(value) >= 1 && codePointLength(value) <= max;

const NAME = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

const nameCheck: Check = (value, directory) => {
  if (typeof value !== 'string') return '"name" is required and must be a string';
  if (!lengthBetween(value, 64) || !NAME.test(value)) {
    return `"name" "${value}" must be 1 to 64 lowercase letters, digits and single hyphens, a letter or digit at each end`;
  }
  return value === directory
    ? undefined
    : `"name" "${value}" must match its directory "${directory}"`;
};

const descriptionCheck: Check = (value) =>
  lengthBetween(value, 1024) && typeof value === 'string' && value.trim() !== ''
    ? undefined
    : '"description" is required: a string of 1 to 1024 characters';

const optionalString =
  (field: string): Check =>
  (value) =>
    typeof value === 'string' ? undefined : `"${field}" must be a string`;

const compatibilityCheck: Check = (value) =>
  lengthBetween(value, 500) ? undefined : '"compatibility" must be a string of 1 to 500 characters';

const metadataCheck: Check = (value) =>
  isRecord(value) && Object.values(value).every((member) => typeof member === 'string')
    ? undefined
    : '"metadata" must map strings to strings';

/** The six Agent Skills fields and their checks; `name` and `description` are checked when absent too. */
const FIELDS: Readonly<Record<string, Check>> = {
  name: nameCheck,
  description: descriptionCheck,
  license: optionalString('license'),
  compatibility: compatibilityCheck,
  metadata: metadataCheck,
  'allowed-tools': optionalString('allowed-tools'),
};

const REQUIRED = new Set(['name', 'description']);

/** Checks a `SKILL.md` found in `skills/<directory>/` by the Agent Skills specification. */
export function checkCoreSkill(text: string, directory: string): CoreSkillCheck {
  const read = parseFrontmatter(text);
  if (!read.present) return { problems: ['SKILL.md has no frontmatter'], unknownFields: [] };
  if (read.problem !== undefined) {
    return {
      problems: [`SKILL.md frontmatter does not parse: ${read.problem}`],
      unknownFields: [],
    };
  }
  const { fields } = read;
  const problems = Object.entries(FIELDS)
    .filter(([field]) => REQUIRED.has(field) || Object.hasOwn(fields, field))
    .map(([field, check]) => check(fields[field], directory))
    .filter((problem): problem is string => problem !== undefined);
  const unknownFields = Object.keys(fields).filter((field) => !Object.hasOwn(FIELDS, field));
  return { problems, unknownFields };
}
