import { coreModule } from '../wasm/bridge.ts';
import type { FieldProblem } from './frontmatter-values.ts';
import type { InvocableFrontmatter } from './skill-types.ts';

/**
 * The documented frontmatter fields a skill and a command share (skills.md, "Frontmatter
 * reference"), each read by its own reader, and the defaults that fill what is not written. A reader
 * takes the raw YAML value and its key and returns what it sets plus the problems with the value; a
 * value the field does not take is dropped with a problem, never thrown (the skill still loads).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/skill_fields.rs` and
 * `skill_field_readers.rs`, docs/RUST.md).
 */

const call = coreModule('format/skill-fields');

/** What one field sets: a piece of the skill, `name` (resolved elsewhere) included. */
export type SkillFieldValues = Partial<InvocableFrontmatter> & { readonly name?: string };

export interface FieldOutcome {
  readonly values: SkillFieldValues;
  readonly problems: readonly FieldProblem[];
}

export type FieldReader = (value: unknown, key: string) => FieldOutcome;

/**
 * The frontmatter keys of a skill, as the table of skills.md ("Frontmatter reference") lists
 * them. "A field name must match the table exactly, hyphens included: Claude Code ignores a field
 * it doesn't recognize without reporting an error", so this list is closed and an unknown key
 * (`when-to-use`, `Description`) is skipped without an issue.
 */
export const SKILL_FIELDS: readonly [
  'name',
  'description',
  'when_to_use',
  'argument-hint',
  'arguments',
  'disable-model-invocation',
  'user-invocable',
  'allowed-tools',
  'disallowed-tools',
  'model',
  'effort',
  'context',
  'agent',
  'background',
  'hooks',
  'paths',
  'shell',
  'metadata',
  'license',
  'compatibility',
] = call('SKILL_FIELDS');

export type SkillField = (typeof SKILL_FIELDS)[number];

/** One reader per documented key (K1: a registry typed over the keys, no switch). */
export const FIELD_READERS: { readonly [K in SkillField]: FieldReader } = call('FIELD_READERS');

/** Run the reader of every documented key that is present; any other key is skipped. */
export const readSkillFields = (fields: Readonly<Record<string, unknown>>): FieldOutcome =>
  call('readSkillFields', fields);

/**
 * The first non-empty line of a Markdown body, trimmed (skills.md: `description` is optional). Read
 * line by line up to that line, so a long body is never split.
 */
export const firstContentLine = (body: string): string | undefined =>
  call('firstContentLine', body);

/**
 * `<` and `>` in `description` and `when_to_use` "show as `&lt;` and `&gt;`" (skills.md, "Angle
 * brackets in a skill description"). The model keeps the text as written; a listing or menu that
 * shows it applies this.
 */
export const escapeAngleBrackets = (text: string): string => call('escapeAngleBrackets', text);

export interface InvocableRead {
  readonly frontmatter: InvocableFrontmatter;
  /** The `name` as declared, not yet resolved to a command name. */
  readonly declaredName?: string;
  readonly problems: readonly FieldProblem[];
}

/**
 * The frontmatter a skill and a command share, with the documented defaults: `disable-model-
 * invocation` false, `user-invocable` true, `background` true, `shell` bash, and the first
 * non-empty body line for a missing `description`. `fields` is kept whole as `frontmatter`; the keys
 * in `untaken` are not read (a command file does not take `name` and `paths`).
 */
export const readInvocable = (
  fields: Readonly<Record<string, unknown>>,
  body: string,
  untaken: readonly string[] = [],
): InvocableRead => call('readInvocable', fields, body, untaken);
