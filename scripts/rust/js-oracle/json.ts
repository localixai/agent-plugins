/** `JSON.parse`, whose error text reaches a plugin's issues (`Invalid JSON syntax: JSON Parse error: …`). */
import { edits, line, outcome, write } from './shared.ts';

const JSON_SEEDS = [
  '{"name":"my-plugin","version":"1.0.0","keywords":["a","b"],"strict":true,"n":-1.5e3}',
  '[1,2.5,-0,1e2,"x\\n\\u00e9\\ud83d\\ude00",null,false,{}]',
  '  {"a" : { "b" : [ ] } }  ',
  '"text"',
  '0',
  // Text beyond ASCII (a string of UTF-16 code units in JavaScriptCore), `__proto__` as a member
  // and a repeated key.
  '{"__proto__":{"k":"\u00e9\u{1F600}\\u0000"},"k":1,"k":[true,null,1E+2]}',
];
const EDITS = [
  '',
  '{',
  '}',
  '[',
  ']',
  '"',
  ',',
  ':',
  '\\',
  'x',
  '0',
  '-',
  '.',
  'e',
  ' ',
  '\u0000',
  'u',
  '+',
  'E',
  't',
  '\t',
  '\u00e9',
  '\ufeff',
];

/** Texts no single edit of a seed reaches: nesting, long numbers, each escape, long words. */
const nest = (depth: number, open: string, inner: string, close: string): string =>
  open.repeat(depth) + inner + close.repeat(depth);
const JSON_CASES = [
  nest(100, '[', '1', ']'),
  nest(100, '{"a":', '1', '}'),
  nest(1000, '[', '1', ''),
  nest(1000, '{"a":', 'x', ''),
  nest(1000, '[', '1', ']') + ']',
  '12345678901234567890123',
  '9007199254740993',
  '-0.000000000000000000000000000000000000001e-300',
  '2.4703282292062328e-324',
  '1.7976931348623157e308',
  '1.7976931348623159e308',
  '-1e400',
  `0.${'1'.repeat(400)}`,
  '"\\"\\\\\\/\\b\\f\\n\\r\\t\\u0041\\u00e9\\ud83d\\ude00"',
  '"\u007f\u0085\u2028"',
  `[${'a'.repeat(300)}]`,
  '[$_a1]',
  '{"a":1,"a":{"b":2},"a":[3]}',
  '{"1":1,"b":2,"0":3}',
];

export const recordJsonParse = (): void => {
  write(
    'json-parse',
    [...edits(JSON_SEEDS, EDITS), ...JSON_CASES].map((text) =>
      line(
        'JSON.parse',
        [text],
        outcome(() => JSON.parse(text) as unknown),
      ),
    ),
  );
};
