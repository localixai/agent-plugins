/**
 * The source rewrite the recorder loads `src/` through, done on the module's syntax tree
 * (oxc-parser), so text that only looks like code (a template that holds JavaScript) is never
 * touched:
 *
 * - every top-level `export function` (and `export async function`, and its overload signatures)
 *   loses its `export` and is renamed `<name>__recorded`, and an exported function declaration of
 *   the same name takes its place: still hoisted, so import cycles behave as before;
 * - a top-level `export const <name> = <arrow or function expression>` loses its `export`, and a
 *   wrapper is exported under its name at the end of the module.
 *
 * Each wrapper keeps its original's `name`, `length` and source text (store.ts, `dress`), and
 * passes every call through `recordCall`. Generators and every other export are left alone.
 */
import { parseSync } from 'oxc-parser';

interface Edit {
  readonly start: number;
  readonly end: number;
  readonly text: string;
}

interface Found {
  readonly declared: string[];
  readonly constants: string[];
  readonly edits: Edit[];
}

interface Span {
  readonly start: number;
  readonly end: number;
}
interface Named extends Span {
  readonly type: string;
  readonly generator?: boolean;
  readonly id?: (Span & { readonly name: string }) | null;
  readonly declarations?: readonly {
    readonly id: Span & { readonly name?: string };
    readonly init?: { readonly type: string } | null;
  }[];
  readonly kind?: string;
}
interface Statement extends Span {
  readonly type: string;
  readonly declaration?: Named | null;
}

const FUNCTION_VALUES = new Set(['ArrowFunctionExpression', 'FunctionExpression']);
const FUNCTION_DECLARATIONS = new Set(['FunctionDeclaration', 'TSDeclareFunction']);

function visit(statement: Statement, found: Found): void {
  const declaration = statement.declaration;
  if (
    statement.type !== 'ExportNamedDeclaration' ||
    declaration === null ||
    declaration === undefined
  )
    return;
  const unexport = { start: statement.start, end: declaration.start, text: '' };
  if (FUNCTION_DECLARATIONS.has(declaration.type) && declaration.generator !== true) {
    const id = declaration.id;
    if (id === null || id === undefined) return;
    found.edits.push(unexport, { start: id.start, end: id.end, text: `${id.name}__recorded` });
    if (!found.declared.includes(id.name)) found.declared.push(id.name);
    return;
  }
  const only = declaration.declarations?.length === 1 ? declaration.declarations[0] : undefined;
  const name = only?.id.name;
  if (
    declaration.kind !== 'const' ||
    name === undefined ||
    !FUNCTION_VALUES.has(only?.init?.type ?? '')
  ) {
    return;
  }
  found.edits.push(unexport);
  found.constants.push(name);
}

/** Offsets as oxc-parser gives them are UTF-16 code units, as JavaScript's string indexes. */
function applyEdits(source: string, edits: readonly Edit[]): string {
  let out = source;
  for (const edit of edits.toSorted((a, b) => b.start - a.start)) {
    out = out.slice(0, edit.start) + edit.text + out.slice(edit.end);
  }
  return out;
}

/** A module switched to the Rust core (docs/RUST.md): its calls are the core's, never recorded. */
const SWITCHED = /\bcallCore\b/u;

export function wrapExports(source: string, module: string, recorder: string): string {
  if (SWITCHED.test(source)) return source;
  const program = parseSync(`${module}.ts`, source).program as { body: readonly Statement[] };
  const found: Found = { declared: [], constants: [], edits: [] };
  for (const statement of program.body) visit(statement, found);
  if (found.declared.length === 0 && found.constants.length === 0) return source;
  const where = JSON.stringify(module);
  const wrappers = [
    ...found.declared.map((name) =>
      [
        `export function ${name}(...args) { return __recordCall(${JSON.stringify(`${module}#${name}`)}, ${name}__recorded, this, args); }`,
        `__dress(${name}, ${name}__recorded, ${JSON.stringify(name)});`,
      ].join('\n'),
    ),
    ...found.constants.map(
      (name) =>
        `const ${name}__wrapped = __wrap(${where}, ${JSON.stringify(name)}, ${name});\nexport { ${name}__wrapped as ${name} };`,
    ),
  ];
  return [
    `import { recordCall as __recordCall, dress as __dress, wrap as __wrap } from ${JSON.stringify(recorder)};`,
    applyEdits(source, found.edits),
    ...wrappers,
  ].join('\n');
}

const BUN_TEST = /from ['"]bun:test['"]/gu;

export function redirectBunTest(source: string, wrapped: string): string {
  return source.replace(BUN_TEST, `from ${JSON.stringify(wrapped)}`);
}
