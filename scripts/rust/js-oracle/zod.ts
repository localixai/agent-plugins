/**
 * zod 4 itself, the package's validator: every schema of `zod-schemas.ts` on every input below,
 * each outcome recorded as `safeParse` answers it (`{ success: true, data }` or `{ success:
 * false, issues }`, every member of every issue in zod's order). The inputs are right values,
 * each wrong type, `undefined`, `null`, `NaN`, `-0`, the infinities, out-of-range and
 * non-integer numbers, empty and spaced strings, characters past U+FFFF, objects with extra keys,
 * index-like keys and an own `__proto__`. `crates/agent-plugins/src/zod.rs` is held to it.
 */
import { ZOD_SCHEMAS } from './zod-schemas.ts';
import { numberText, write } from './shared.ts';

const ON_SKILL = 'on-skill-invoke:';

const SCALARS: readonly unknown[] = [
  undefined,
  null,
  true,
  false,
  0,
  -0,
  1,
  1.5,
  -1,
  65_536,
  Number.NaN,
  Number.POSITIVE_INFINITY,
  Number.NEGATIVE_INFINITY,
  2 ** 53,
  -(2 ** 53),
  '',
  'a',
  'ab',
  'abcde',
  ' x ',
  'two words',
  '/bin/two words',
  '.ts',
  'a@b@c',
  '😀😀',
  'always',
  ON_SKILL,
  `${ON_SKILL}review`,
  'acceptEdits',
  'command',
  'stdio',
  'https://example.com',
  ' https://a.b/c ',
  'http://plain.example',
  'not a url',
];

/** Objects and arrays, read from JSON so `__proto__` is an own member as a file gives it. */
const OBJECTS: readonly unknown[] = [
  '[]',
  '["a"]',
  '["a","b","c"]',
  '[1,"a"]',
  '[null]',
  '[["a"],{"x":"y"}]',
  '[{"name":"a"},{"name":"b"}]',
  '[{"name":"a"},{"name":"a"}]',
  '[{"name":""},{"name":"b","extra":1}]',
  '{}',
  '{"length":3}',
  '{"length":"7"}',
  '{"name":"n"}',
  '{"name":1}',
  '{"name":"n","count":2,"extra":true}',
  '{"name":"n","flag":"yes"}',
  '{"name":"n","__proto__":{"x":1}}',
  '{"1":"x","b":"y","0":"z","2":"w"}',
  '{"name":null}',
  '{"value":null}',
  '{"a":1,"b":"c"}',
  '{"a":"x","b":"y"}',
  '{".ts":"typescript"}',
  '{"ts":"typescript",".js":2}',
  '{"type":"setMode","mode":"m"}',
  '{"type":"addDirectories","directories":["d",3]}',
  '{"type":"other"}',
  '{"type":5}',
  '{"kind":"a","a":"x"}',
  '{"kind":"b","b":"x"}',
  '{"source":"s"}',
  '{"source":"s","content":"c"}',
  '{"author":{"name":1},"tags":[1]}',
  '{"author":{"name":"me","email":"e"},"tags":["t"],"more":1}',
  '{"type":"number","default":"1"}',
  '{"type":"string","default":"1","min":0}',
  '{"url":" https://x.y ","headers":{"a":1}}',
  '{"url":"nope"}',
  '{"mode":"b","count":"x"}',
  '{"count":"x","name":3}',
  '{"entry":{"type":"addRules"}}',
  '{"entry":{"kind":1},"value":0}',
  '{"version":"1"}',
  '{"name":"n","b":2}',
].map((text) => JSON.parse(text));

/** A value as the vectors spell it: JSON, `undefined` and the numbers JSON lacks tagged, a member
 * set to `undefined` kept (an output's present-but-undefined member is part of what zod gives). */
function wire(value: unknown): unknown {
  if (value === undefined) return { $undefined: true };
  if (typeof value === 'number') return numberText(value);
  if (Array.isArray(value)) return value.map(wire);
  if (typeof value !== 'object' || value === null) return value;
  const out = {};
  for (const [key, item] of Object.entries(value)) {
    Object.defineProperty(out, key, {
      value: wire(item),
      enumerable: true,
      writable: true,
      configurable: true,
    });
  }
  return out;
}

const outcome = (name: string, input: unknown): unknown => {
  const parsed = ZOD_SCHEMAS[name]?.safeParse(input);
  if (parsed === undefined) return { throws: `no schema ${name}` };
  return parsed.success
    ? { result: { success: true, data: wire(parsed.data) } }
    : { result: { success: false, issues: wire(parsed.error.issues) } };
};

const line = (name: string, input: unknown): string =>
  JSON.stringify({ fn: name, args: [wire(input)], outcomes: [outcome(name, input)], spec: [] });

/** `error.message` of a few failures: the text `parse` throws. */
const MESSAGES: readonly (readonly [string, unknown])[] = [
  ['string', 1],
  ['object.nested', JSON.parse('{"author":{"name":1},"tags":[1]}')],
  ['union.objects', {}],
];

function message(name: string, input: unknown): string {
  let text = '';
  try {
    ZOD_SCHEMAS[name]?.parse(input);
  } catch (error) {
    text = error instanceof Error ? error.message : String(error);
  }
  return JSON.stringify({
    fn: 'message',
    args: [name, wire(input)],
    outcomes: [{ result: text }],
    spec: [],
  });
}

export const recordZod = (): void => {
  const names = Object.keys(ZOD_SCHEMAS);
  write('zod', [
    ...names.flatMap((name) => [...SCALARS, ...OBJECTS].map((input) => line(name, input))),
    ...MESSAGES.map(([name, input]) => message(name, input)),
  ]);
};
