// Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.
import { joinTables, type ShapeTable } from '../tool-shape.ts';
import { SHAPES_0 } from './shapes-0.ts';
import { SHAPES_1 } from './shapes-1.ts';
import { SHAPES_2 } from './shapes-2.ts';
import { SHAPES_3 } from './shapes-3.ts';
import { SHAPES_4 } from './shapes-4.ts';
import { SHAPES_5 } from './shapes-5.ts';

/** The output shape of every built-in tool the SDK types, by tool name. */
export const TOOL_SHAPES: ShapeTable = joinTables([
  SHAPES_0,
  SHAPES_1,
  SHAPES_2,
  SHAPES_3,
  SHAPES_4,
  SHAPES_5,
]);
