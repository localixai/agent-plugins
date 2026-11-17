/**
 * The shape of a built-in tool's output as `sdk-tools.d.ts` types it (Agent Plugins Extended J7.138:
 * a `PostToolUse` hook's `updatedToolOutput` is checked against the tool's output type), as data the
 * kit can test a value against. `scripts/tool-output-shapes.ts` writes the tables of
 * `tool-shapes/` from the SDK's file; a test checks they are current.
 *
 * A primitive is named; `literal` is one value; `array` its element; `object` its fields by name,
 * a name ending in `?` an optional field, and any other key allowed, as the SDK's types are open;
 * `oneOf` its alternatives. A type the file declares as `unknown` takes any value.
 */
export type Shape =
  | PrimitiveShape
  | { readonly literal: string | number | boolean }
  | { readonly array: Shape }
  | { readonly object: Readonly<Record<string, Shape>> }
  | { readonly oneOf: readonly Shape[] };

export type PrimitiveShape = 'string' | 'number' | 'boolean' | 'null' | 'unknown';

/** The output shape of each built-in tool, by the tool's name. */
export type ShapeTable = Readonly<Record<string, Shape>>;

const alternatives = (shape: Shape | undefined): readonly Shape[] =>
  shape === undefined ? [] : typeof shape === 'object' && 'oneOf' in shape ? shape.oneOf : [shape];

/** The tables as one; a tool in several (its alternatives split to keep each file short) has them all. */
export function joinTables(tables: readonly ShapeTable[]): ShapeTable {
  const joined: Record<string, Shape> = {};
  for (const table of tables) {
    for (const [tool, shape] of Object.entries(table)) {
      joined[tool] = Object.hasOwn(joined, tool)
        ? { oneOf: [...alternatives(joined[tool]), ...alternatives(shape)] }
        : shape;
    }
  }
  return joined;
}

const PRIMITIVES: { readonly [P in PrimitiveShape]: (value: unknown) => boolean } = {
  string: (value) => typeof value === 'string',
  number: (value) => typeof value === 'number',
  boolean: (value) => typeof value === 'boolean',
  null: (value) => value === null,
  unknown: () => true,
};

const isPrimitive = (shape: Shape): shape is PrimitiveShape => typeof shape === 'string';

const describe = (value: unknown): string =>
  value === null ? 'null' : Array.isArray(value) ? 'array' : typeof value;

const at = (path: string, why: string): string => (path === '' ? why : `${path}: ${why}`);

const join = (path: string, key: string | number): string =>
  path === '' ? String(key) : `${path}.${String(key)}`;

const OPTIONAL = '?';

function objectProblem(
  fields: Readonly<Record<string, Shape>>,
  value: unknown,
  path: string,
): string | undefined {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return at(path, `expected an object, received ${describe(value)}`);
  }
  for (const [written, shape] of Object.entries(fields)) {
    const optional = written.endsWith(OPTIONAL);
    const key = optional ? written.slice(0, -1) : written;
    const own: unknown = Reflect.get(value, key);
    if (own === undefined) {
      if (optional) continue;
      return at(join(path, key), 'required');
    }
    const problem = shapeProblem(shape, own, join(path, key));
    if (problem !== undefined) return problem;
  }
  return undefined;
}

function arrayProblem(element: Shape, value: unknown, path: string): string | undefined {
  if (!Array.isArray(value)) return at(path, `expected an array, received ${describe(value)}`);
  for (const [index, item] of value.entries()) {
    const problem = shapeProblem(element, item, join(path, index));
    if (problem !== undefined) return problem;
  }
  return undefined;
}

/** The first place `value` breaks `shape`, as `<path>: <why>`; `undefined` when it fits. */
export function shapeProblem(shape: Shape, value: unknown, path = ''): string | undefined {
  if (isPrimitive(shape)) {
    return PRIMITIVES[shape](value)
      ? undefined
      : at(path, `expected ${shape}, received ${describe(value)}`);
  }
  if ('literal' in shape) {
    return value === shape.literal
      ? undefined
      : at(path, `expected ${JSON.stringify(shape.literal)}`);
  }
  if ('array' in shape) return arrayProblem(shape.array, value, path);
  if ('object' in shape) return objectProblem(shape.object, value, path);
  const fits = shape.oneOf.some((one) => shapeProblem(one, value, path) === undefined);
  return fits ? undefined : at(path, 'matches none of the shapes the type allows');
}
