/**
 * Writes `src/hooks/output/tool-shapes/`: the output shape of every built-in tool, read from the
 * official `sdk-tools.d.ts` of the Agent SDK the spec targets (E2; Agent Plugins Extended J7.138: a
 * `PostToolUse` hook's `updatedToolOutput` is checked against the tool's output type). Each member of
 * `ToolOutputSchemas` is one tool; the MCP output (`McpOutput`) is not, as MCP tool output passes
 * unchecked. `tests/hooks/output/tool-shapes.test.ts` fails when the tables are stale.
 *
 *   bun scripts/tool-output-shapes.ts
 */
import { join } from 'node:path';
import { parseSync } from 'oxc-parser';
import type { Shape, ShapeTable } from '../src/hooks/output/tool-shape.ts';

export const SDK_TOOLS = join(
  import.meta.dir,
  '..',
  'node_modules',
  '@anthropic-ai',
  'claude-agent-sdk',
  'sdk-tools.d.ts',
);
export const SHAPES_DIR = join(import.meta.dir, '..', 'src', 'hooks', 'output', 'tool-shapes');

/** The output types whose tool's name is not the type's name without `Output`. */
const TOOL_OF: Readonly<Record<string, string | null>> = {
  FileReadOutput: 'Read',
  FileEditOutput: 'Edit',
  FileWriteOutput: 'Write',
  ListMcpResourcesOutput: 'ListMcpResourcesTool',
  ReadMcpResourceOutput: 'ReadMcpResourceTool',
  McpOutput: null,
};

// The AST of oxc-parser is untyped JSON here: each node is read by its `type`.
type Node = { readonly type: string; readonly [key: string]: unknown };

const node = (value: unknown): Node => value as Node;
const nodes = (value: unknown): readonly Node[] => value as Node[];

class Reader {
  private readonly decls: ReadonlyMap<string, Node>;

  constructor(decls: ReadonlyMap<string, Node>) {
    this.decls = decls;
  }

  private fields(members: readonly Node[], seen: ReadonlySet<string>): Shape {
    const object: Record<string, Shape> = {};
    for (const member of members) {
      if (member.type !== 'TSPropertySignature') continue;
      const key = node(member.key);
      const name = String(key.name ?? key.value);
      const shape = this.shape(node(node(member.typeAnnotation).typeAnnotation), seen);
      object[member.optional === true ? `${name}?` : name] = shape;
    }
    return { object };
  }

  private reference(name: string, seen: ReadonlySet<string>): Shape {
    const decl = this.decls.get(name);
    if (decl === undefined || seen.has(name)) return 'unknown';
    const next = new Set([...seen, name]);
    return decl.type === 'TSInterfaceDeclaration'
      ? this.fields(nodes(node(decl.body).body), next)
      : this.shape(node(decl.typeAnnotation), next);
  }

  private tuple(elements: readonly Node[], seen: ReadonlySet<string>): Shape {
    const shapes = elements.map((element) => {
      if (element.type === 'TSRestType') {
        const rest = this.shape(node(element.typeAnnotation), seen);
        return typeof rest === 'object' && 'array' in rest ? rest.array : rest;
      }
      const inner = element.type === 'TSOptionalType' ? node(element.typeAnnotation) : element;
      return this.shape(inner, seen);
    });
    const distinct = shapes.filter(
      (one, index) =>
        shapes.findIndex((other) => JSON.stringify(other) === JSON.stringify(one)) === index,
    );
    return distinct.length === 1 ? (distinct[0] ?? 'unknown') : { oneOf: distinct };
  }

  shape(type: Node, seen: ReadonlySet<string> = new Set()): Shape {
    const keyword: Readonly<Record<string, Shape>> = {
      TSStringKeyword: 'string',
      TSNumberKeyword: 'number',
      TSBooleanKeyword: 'boolean',
      TSNullKeyword: 'null',
      TSUnknownKeyword: 'unknown',
      TSAnyKeyword: 'unknown',
    };
    const named = keyword[type.type];
    if (named !== undefined) return named;
    if (type.type === 'TSLiteralType') {
      return { literal: node(type.literal).value as string | number | boolean };
    }
    if (type.type === 'TSParenthesizedType') return this.shape(node(type.typeAnnotation), seen);
    if (type.type === 'TSArrayType') return { array: this.shape(node(type.elementType), seen) };
    if (type.type === 'TSTypeLiteral') return this.fields(nodes(type.members), seen);
    if (type.type === 'TSUnionType') {
      return { oneOf: nodes(type.types).map((one) => this.shape(one, seen)) };
    }
    if (type.type === 'TSTypeReference') {
      return this.reference(String(node(type.typeName).name), seen);
    }
    // A tuple (json-schema-to-typescript's `minItems`): read as an array of what its elements may be.
    if (type.type === 'TSTupleType') return { array: this.tuple(nodes(type.elementTypes), seen) };
    throw new Error(`sdk-tools.d.ts: no shape for a ${type.type}`);
  }
}

/** Every built-in tool's output shape, by tool name, in the order `ToolOutputSchemas` lists them. */
export function readToolShapes(source: string): ShapeTable {
  const { program } = parseSync('sdk-tools.d.ts', source);
  const decls = new Map<string, Node>();
  for (const statement of nodes(node(program).body)) {
    const decl =
      statement.type === 'ExportNamedDeclaration' ? node(statement.declaration) : statement;
    if (decl?.type === 'TSInterfaceDeclaration' || decl?.type === 'TSTypeAliasDeclaration') {
      decls.set(String(node(decl.id).name), decl);
    }
  }
  const reader = new Reader(decls);
  const outputs = decls.get('ToolOutputSchemas');
  if (outputs === undefined) throw new Error('sdk-tools.d.ts: no ToolOutputSchemas');
  const table: Record<string, Shape> = {};
  for (const member of nodes(node(outputs.typeAnnotation).types)) {
    const type = String(node(member.typeName).name);
    const tool = Object.hasOwn(TOOL_OF, type) ? TOOL_OF[type] : type.replace(/Output$/, '');
    if (tool === null || tool === undefined) continue;
    table[tool] = reader.shape(member);
  }
  return table;
}

/** How many lines a shape takes once formatted, to keep each table file under S1's 300. */
function weight(tool: string, shape: Shape): number {
  const text = `const x = { ${JSON.stringify(tool)}: ${JSON.stringify(shape)} };\n`;
  const formatted = Bun.spawnSync(['bunx', 'oxfmt', '--stdin-filepath', 'x.ts'], {
    stdin: Buffer.from(text),
  });
  return formatted.stdout.toString().split('\n').length - 3;
}
const BUDGET = 270;

type Piece = readonly [tool: string, shape: Shape];

/** A tool whose alternatives do not fit one table is split into pieces, which `joinTables` puts back together. */
function piecesOf(tool: string, shape: Shape): readonly Piece[] {
  if (weight(tool, shape) <= BUDGET || typeof shape !== 'object' || !('oneOf' in shape)) {
    return [[tool, shape]];
  }
  const half = Math.ceil(shape.oneOf.length / 2);
  return [
    ...piecesOf(tool, { oneOf: shape.oneOf.slice(0, half) }),
    ...piecesOf(tool, { oneOf: shape.oneOf.slice(half) }),
  ];
}

/** The tools split into tables of at most `BUDGET` lines, in order; two pieces of a tool never share one. */
export function groupsOf(table: ShapeTable): readonly (readonly Piece[])[] {
  const groups: Piece[][] = [[]];
  let used = 0;
  for (const piece of Object.entries(table).flatMap(([tool, shape]) => piecesOf(tool, shape))) {
    const cost = weight(...piece);
    const current = groups.at(-1) ?? [];
    const clash = current.some(([tool]) => tool === piece[0]);
    if ((used + cost > BUDGET || clash) && current.length > 0) {
      groups.push([]);
      used = 0;
    }
    groups.at(-1)?.push(piece);
    used += cost;
  }
  return groups;
}

const HEADER =
  '// Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.\n';

function fileText(index: number, pieces: readonly Piece[]): string {
  const body = Object.fromEntries(pieces);
  return `${HEADER}import type { ShapeTable } from '../tool-shape.ts';\n\nexport const SHAPES_${index}: ShapeTable = ${JSON.stringify(body)};\n`;
}

function indexText(count: number): string {
  const names = Array.from({ length: count }, (_, index) => `SHAPES_${index}`);
  const imports = names.map((name, index) => `import { ${name} } from './shapes-${index}.ts';`);
  return `${HEADER}import { joinTables, type ShapeTable } from '../tool-shape.ts';\n${imports.join('\n')}\n\n/** The output shape of every built-in tool the SDK types, by tool name. */\nexport const TOOL_SHAPES: ShapeTable = joinTables([${names.join(', ')}]);\n`;
}

if (import.meta.main) {
  const table = readToolShapes(await Bun.file(SDK_TOOLS).text());
  const groups = groupsOf(table);
  const written: string[] = [];
  for (const [index, pieces] of groups.entries()) {
    const path = join(SHAPES_DIR, `shapes-${index}.ts`);
    await Bun.write(path, fileText(index, pieces));
    written.push(path);
  }
  const index = join(SHAPES_DIR, 'index.ts');
  await Bun.write(index, indexText(groups.length));
  const format = Bun.spawnSync(['bunx', 'oxfmt', '--write', index, ...written]);
  if (format.exitCode !== 0) throw new Error(format.stderr.toString());
  console.log(`${Object.keys(table).length} tools in ${groups.length} tables: ${SHAPES_DIR}`);
}
