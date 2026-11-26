/**
 * The values that cross into the Rust core and back (docs/RUST.md, "The boundary"): what JSON has
 * no spelling for is tagged, at every depth, both ways, so JavaScript's own values reach the core
 * whole: `undefined`, and the numbers `NaN`, `Infinity`, `-Infinity` and `-0`.
 */
import { expect, test } from 'bun:test';
import { canonicalJson } from '../../src/format/canonical-json.ts';
import { readInteger, readTable } from '../../src/format/frontmatter-values.ts';
import { decodeAnswer, encodeArgs } from '../../src/wasm/wire.ts';

test('the four numbers JSON lacks are tagged at every depth', () => {
  const args = [Number.NaN, [Infinity, { a: -Infinity, b: -0 }], 0, 1.5];
  expect(encodeArgs(args)).toBe(
    '[{"$number":"NaN"},[{"$number":"Infinity"},{"a":{"$number":"-Infinity"},"b":{"$number":"-0"}}],0,1.5]',
  );
});

test('undefined is tagged as an argument, an array item and an object member', () => {
  expect(encodeArgs([undefined, [undefined], { a: undefined }])).toBe(
    '[{"$undefined":true},[{"$undefined":true}],{"a":{"$undefined":true}}]',
  );
});

test('a tag is read back as its value, at every depth', () => {
  const answer = decodeAnswer(
    '{"result":[{"$number":"NaN"},{"a":{"$number":"-0"},"b":{"$undefined":true}},{"$number":"x"}]}',
  );
  // Strict: `b` is a member set to `undefined`, not a missing one, and `-0` is not `0`.
  expect(answer).toStrictEqual({
    result: [Number.NaN, { a: -0, b: undefined }, { $number: 'x' }],
  });
  expect(answer).not.toStrictEqual({
    result: [Number.NaN, { a: 0, b: undefined }, { $number: 'x' }],
  });
});

test('the core reads the numbers as JavaScript holds them', () => {
  expect(readInteger('maxTurns', Number.NaN).problem?.reason).toBe('is not a whole number');
  expect(readInteger('maxTurns', Infinity).problem?.reason).toBe('is not a whole number');
  expect(Object.is(readInteger('maxTurns', -0).value, -0)).toBe(true);
  expect(readInteger('k', 5, { max: Infinity, min: -Infinity }).value).toBe(5);
  expect(canonicalJson([Number.NaN, -0, undefined, { a: undefined, b: Infinity }])).toBe(
    '[null,0,null,{"b":null}]',
  );
  const table = readTable('k', { a: Number.NaN, b: undefined }).value;
  expect(Number.isNaN(table?.a)).toBe(true);
  expect(table !== undefined && 'b' in table).toBe(true);
});
