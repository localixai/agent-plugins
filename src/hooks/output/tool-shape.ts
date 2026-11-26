/**
 * The shape of a built-in tool's output as `sdk-tools.d.ts` types it (Agent Plugins Extended J7.138:
 * a `PostToolUse` hook's `updatedToolOutput` is checked against the tool's output type), as data the
 * kit can test a value against. `scripts/tool-output-shapes.ts` writes the tables of
 * `tool-shapes/` from the SDK's file; a test checks they are current.
 *
 * A primitive is named; `literal` is one value; `array` its element; `object` its fields by name,
 * a name ending in `?` an optional field, and any other key allowed, as the SDK's types are open;
 * `oneOf` its alternatives. A type the file declares as `unknown` takes any value.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/tool_shape.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

export type Shape =
  | PrimitiveShape
  | { readonly literal: string | number | boolean }
  | { readonly array: Shape }
  | { readonly object: Readonly<Record<string, Shape>> }
  | { readonly oneOf: readonly Shape[] };

export type PrimitiveShape = 'string' | 'number' | 'boolean' | 'null' | 'unknown';

/** The output shape of each built-in tool, by the tool's name. */
export type ShapeTable = Readonly<Record<string, Shape>>;

const call = coreModule('hooks/output/tool-shape');

/** The tables as one; a tool in several (its alternatives split to keep each file short) has them all. */
export const joinTables = (tables: readonly ShapeTable[]): ShapeTable => call('joinTables', tables);

/** The first place `value` breaks `shape`, as `<path>: <why>`; `undefined` when it fits. */
export const shapeProblem = (shape: Shape, value: unknown, path = ''): string | undefined =>
  call('shapeProblem', shape, value, path);
