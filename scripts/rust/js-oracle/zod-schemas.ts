/**
 * The schemas the zod oracle runs (`zod.ts`), one per builder and method the package uses and
 * the combinations it writes, each by name. `crates/agent-plugins/tests/differential/js/zod_schemas.rs`
 * builds the same schemas in Rust under the same names in the same order: a schema changed here
 * is changed there.
 */
import { z } from 'zod';

const isHttps = (text: string): boolean =>
  URL.canParse(text) && new URL(text).protocol === 'https:';
const wrongValue =
  (expected: string) =>
  (issue: { readonly input?: unknown }): string =>
    issue.input === undefined ? 'is required' : `expected ${expected}`;
const closedObject = <const Shape extends z.ZodRawShape>(shape: Shape) => {
  const known = new Set(Object.keys(shape));
  return z.looseObject(shape).check((context) => {
    const keys = Object.keys(context.value).filter((key) => !known.has(key));
    if (keys.length === 0) return;
    context.issues.push({ code: 'unrecognized_keys', keys, input: context.value, continue: true });
  });
};
const MODES = ['default', 'acceptEdits'];
const ON_SKILL = 'on-skill-invoke:';
const skillOf = (when: string): string =>
  when.startsWith(ON_SKILL) ? when.slice(ON_SKILL.length) : '';

export const ZOD_SCHEMAS: Readonly<Record<string, z.ZodType>> = {
  string: z.string(),
  number: z.number(),
  boolean: z.boolean(),
  unknown: z.unknown(),
  int: z.int(),
  url: z.url(),
  json: z.json(),
  literal: z.literal('command'),
  'literal.number': z.literal(0),
  'literal.error': z.literal('text', { error: 'expected "text"' }),
  enum: z.enum(['stdio', 'socket']),
  'enum.one': z.enum(['only']),
  nullable: z.string().nullable(),
  optional: z.string().optional(),
  exactOptional: z.string().exactOptional(),
  'string.min': z.string().min(1),
  'string.max': z.string().max(3),
  'string.min.max': z.string().min(2).max(4),
  'string.min.error': z.string().min(1, { error: 'expected a string' }),
  'string.regex': z.string().regex(/^\./),
  'string.regex.key': z.string().regex(/^[A-Za-z_][A-Za-z0-9_]*$/),
  'number.positive': z.number().positive(),
  'number.nonnegative': z.number().nonnegative(),
  'number.max': z.number().max(10),
  'number.int': z.number().int(),
  'number.int.positive': z.number().int().positive(),
  'number.int.min': z.number().int().min(0),
  'int.min.max': z.int().min(1).max(65535),
  'int.nonnegative': z.int().nonnegative(),
  'number.errors': z
    .number({ error: 'must be a positive number' })
    .positive({ error: 'must be a positive number' })
    .finite({ error: 'must be a positive number' }),
  array: z.array(z.string()),
  'array.min': z.array(z.string()).min(1),
  'array.max': z.array(z.string().max(3)).max(2),
  'array.method': z.string().array(),
  record: z.record(z.string(), z.string()),
  'record.unknown': z.record(z.string(), z.unknown()),
  'record.key-regex': z
    .record(z.string().regex(/^\./), z.string())
    .refine((map) => Object.keys(map).length > 0, { error: 'needs one mapping' }),
  'record.enum-values': z.record(z.string(), z.enum(['a', 'b'])),
  'record.enum-keys': z.record(z.enum(['a', 'b']), z.number()),
  object: z.object({ name: z.string(), count: z.number().optional() }),
  'object.loose': z.looseObject({ name: z.string(), flag: z.boolean().optional() }),
  'object.strict': z.strictObject({ name: z.string() }),
  'object.unknown': z.object({ value: z.unknown() }),
  'object.exact': z.object({
    name: z.string().exactOptional(),
    value: z.unknown().exactOptional(),
  }),
  'object.nullable': z.object({ name: z.string().nullable(), b: z.string().optional() }),
  'object.nested': z.object({
    author: z.object({ name: z.string(), email: z.string().optional() }),
    tags: z.array(z.string()),
  }),
  'object.partial': z.object({ name: z.string(), version: z.string() }).partial(),
  'object.required': z
    .object({ name: z.string(), version: z.string() })
    .partial()
    .required({ name: true }),
  'object.extend': z.object({ name: z.string() }).extend({ b: z.number() }),
  'object.indexes': z.object({ '2': z.string(), b: z.string().optional(), '1': z.string() }),
  'object.refine-path': z
    .object({ source: z.string().optional(), content: z.string().optional() })
    .refine((entry) => (entry.source === undefined) !== (entry.content === undefined), {
      message: 'exactly one of source or content',
      path: ['source'],
    }),
  'object.refine': z
    .object({ source: z.string().optional(), content: z.string().optional() })
    .refine((entry) => (entry.source === undefined) !== (entry.content === undefined), {
      message: 'exactly one of source or content',
    }),
  'object.error-fn': z.object(
    {
      name: z.string({ error: wrongValue('a string') }),
      count: z.number({ error: wrongValue('a number') }).optional(),
      tags: z.array(z.string(), { error: wrongValue('an array') }).optional(),
      mode: z
        .enum(['a', 'b'], {
          error: (issue) => (issue.input === undefined ? 'is required' : 'expected one of a, b'),
        })
        .optional(),
    },
    { error: (issue) => (issue.input === undefined ? 'is required' : 'expected an object') },
  ),
  union: z.union([z.string(), z.array(z.string())]),
  'union.objects': z.union([
    z.object({ kind: z.literal('a'), a: z.string() }),
    z.object({ kind: z.literal('b'), b: z.number() }),
  ]),
  'union.refined-list': z.union([
    z.string(),
    z
      .array(closedObject({ name: z.string().min(1), when: z.string().exactOptional() }))
      .refine((entries) => new Set(entries.map((entry) => entry.name)).size === entries.length, {
        error: 'names must be unique',
      }),
  ]),
  'union.nested': z.union([
    z.string(),
    z.record(z.string(), z.string()),
    z.array(z.union([z.string(), z.record(z.string(), z.string())])),
  ]),
  'union.scalars': z.union([z.string(), z.number(), z.boolean(), z.array(z.string())]).optional(),
  discriminated: z.discriminatedUnion('type', [
    z.object({ type: z.literal('setMode'), mode: z.string() }),
    z.object({ type: z.literal('addDirectories'), directories: z.array(z.string()) }),
  ]),
  'refine.when': z
    .string()
    .refine((value) => value === 'always' || value.startsWith(ON_SKILL), {
      error: `Invalid string: must start with "${ON_SKILL}"`,
    })
    .refine((value) => value === 'always' || skillOf(value).length > 0, {
      error: 'on-skill-invoke: must specify a skill name',
    }),
  'min.refine': z
    .string()
    .min(1)
    .refine((value) => value.startsWith('/') || !/\s/.test(value), {
      error: 'Command should not contain spaces.',
    }),
  'refine.no-params': z.string().refine((text) => text.indexOf('@') === text.lastIndexOf('@')),
  superRefine: z
    .strictObject({ type: z.enum(['string', 'number']), default: z.string().optional() })
    .superRefine((option, context) => {
      if (option.type === 'number' && option.default !== undefined) {
        context.addIssue({
          code: 'custom',
          path: ['default'],
          message: 'default must be a number',
        });
      }
    }),
  'check.unrecognized': closedObject({ name: z.string().min(1), when: z.string().exactOptional() }),
  'check.custom': z.string().check((context) => {
    if (context.value.trim() === context.value) return;
    context.issues.push({
      code: 'custom',
      input: context.value,
      message: 'An option cannot start or end with a space',
      continue: true,
    });
  }),
  transform: z.string().transform((raw, context) => {
    if (MODES.includes(raw)) return raw;
    context.addIssue({ code: 'custom', message: `"${raw}" is not a permission mode` });
    return z.NEVER;
  }),
  'transform.length': z.string().transform((text) => text.length),
  default: z.string().default('x'),
  'default.object': z.object({
    mode: z.enum(['a', 'b']).default('a'),
    count: z.number().default(1),
  }),
  catch: z.number().catch(0),
  'catch.object': z.object({
    count: z.number().catch(-1),
    name: z.string().optional().catch('none'),
  }),
  custom: z.custom((value) => value !== undefined, { message: 'Required' }),
  'custom.object': z.object({
    value: z.custom((value) => value !== undefined, { message: 'Required' }).exactOptional(),
    entry: z.custom((value) => z.looseObject({ type: z.string() }).safeParse(value).success, {
      message: 'Expected a permission update entry',
    }),
  }),
  'refine.optional': z.string().refine(isHttps, { error: 'must use https://' }).optional(),
  'url.object': z.object({ url: z.url(), headers: z.record(z.string(), z.string()).optional() }),
  instanceof: z.instanceof(Array),
};
