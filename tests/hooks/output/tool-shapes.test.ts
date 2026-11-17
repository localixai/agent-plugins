/**
 * The output shapes `PostToolUse` checks a built-in tool's `updatedToolOutput` against (Agent
 * Plugins Extended J7.138) are the SDK's `sdk-tools.d.ts`, read by `scripts/tool-output-shapes.ts`:
 * the tables in `src/hooks/output/tool-shapes/` must be what that script reads today. Tested outside
 * `spec(...)`: it proves the data is current, not a behaviour.
 */
import { expect, test } from 'bun:test';
import { readToolShapes, SDK_TOOLS } from '../../../scripts/tool-output-shapes.ts';
import { shapeProblem } from '../../../src/hooks/output/tool-shape.ts';
import { TOOL_SHAPES } from '../../../src/hooks/output/tool-shapes/index.ts';

test('the shape tables are the ones sdk-tools.d.ts gives (run `bun scripts/tool-output-shapes.ts` when not)', async () => {
  expect(TOOL_SHAPES).toEqual(readToolShapes(await Bun.file(SDK_TOOLS).text()));
});

test('a problem names the path of the first field that breaks the shape', () => {
  const shape = { object: { a: { array: 'number' }, 'b?': { literal: 'x' } } } as const;
  expect(shapeProblem(shape, { a: [1, 2] })).toBeUndefined();
  expect(shapeProblem(shape, { a: [1, 'two'] })).toBe('a.1: expected number, received string');
  expect(shapeProblem(shape, { a: [], b: 'y' })).toBe('b: expected "x"');
  expect(shapeProblem(shape, {})).toBe('a: required');
  expect(shapeProblem(shape, [])).toBe('expected an object, received array');
});
