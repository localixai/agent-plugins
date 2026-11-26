/**
 * The output shapes `PostToolUse` checks a built-in tool's `updatedToolOutput` against (Agent
 * Plugins Extended J7.138) are the SDK's `sdk-tools.d.ts`, read by `scripts/tool-output-shapes.ts`:
 * the core's tables (`crates/agent-plugins/src/hooks/output/tool_shapes.rs` and `tool_shapes/`) must
 * be what that script writes today, file for file. Tested outside `spec(...)`: it proves the data is
 * current, not a behaviour.
 */
import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { expect, test } from 'bun:test';
import {
  readToolShapes,
  SDK_TOOLS,
  SHAPES_DIR,
  shapeFiles,
} from '../../../scripts/tool-output-shapes.ts';
import { CHECKED_TOOL_OUTPUTS } from '../../../src/hooks/output/tool-output.ts';
import { shapeProblem } from '../../../src/hooks/output/tool-shape.ts';

test("the core's shape tables are the ones sdk-tools.d.ts gives (run `bun scripts/tool-output-shapes.ts` when not)", async () => {
  const table = readToolShapes(await Bun.file(SDK_TOOLS).text());
  const files = shapeFiles(table);
  const written = readdirSync(join(SHAPES_DIR, 'tool_shapes')).map((name) => `tool_shapes/${name}`);
  expect(['tool_shapes.rs', ...written].toSorted()).toEqual([...files.keys()].toSorted());
  for (const [path, text] of files)
    expect(await Bun.file(join(SHAPES_DIR, path)).text()).toBe(text);
  expect(CHECKED_TOOL_OUTPUTS).toEqual(Object.keys(table));
});

test('a problem names the path of the first field that breaks the shape', () => {
  const shape = { object: { a: { array: 'number' }, 'b?': { literal: 'x' } } } as const;
  expect(shapeProblem(shape, { a: [1, 2] })).toBeUndefined();
  expect(shapeProblem(shape, { a: [1, 'two'] })).toBe('a.1: expected number, received string');
  expect(shapeProblem(shape, { a: [], b: 'y' })).toBe('b: expected "x"');
  expect(shapeProblem(shape, {})).toBe('a: required');
  expect(shapeProblem(shape, [])).toBe('expected an object, received array');
});
