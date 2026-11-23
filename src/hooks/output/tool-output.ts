import { isRecord } from '../../protocol/is-record.ts';
import { shapeProblem, type Shape } from './tool-shape.ts';
import { TOOL_SHAPES } from './tool-shapes/index.ts';

/**
 * The output shape of the built-in tools a `PostToolUse` hook may replace (hooks.md, "PostToolUse
 * decision control"): "For built-in tools, a value that doesn't match the tool's output schema is
 * ignored and the original output is used. MCP tool output is passed through without schema
 * validation." Agent Plugins Extended J7.138: the shape is the tool's output type in the SDK's
 * `sdk-tools.d.ts`, where one is defined; a tool without one passes unchecked. The shapes are read
 * from that file (`tool-shapes/`, written by `scripts/tool-output-shapes.ts`), every nested field
 * included; a field the type does not declare is allowed, as the types are open.
 */

/**
 * Code Mode's `REPL` tool, which the SDK types no output for (Agent Plugins Extended J7.214). The
 * client "accepts the copy only in this shape" (§7.6.11.3 item 2, code-mode.md "Redact code
 * output"): `code`, `stdout` and `stderr` strings, `error` a string or absent, `result` any value or
 * missing, `images` and `documents` lists (`[]` removes them).
 */
const REPL_SHAPE: Shape = {
  object: {
    code: 'string',
    stdout: 'string',
    stderr: 'string',
    'error?': 'string',
    'result?': 'unknown',
    'images?': { array: 'unknown' },
    'documents?': { array: 'unknown' },
  },
};

/** The shapes the spec states for tools the SDK gives no output type. */
const SPEC_SHAPES: Readonly<Record<string, Shape>> = { REPL: REPL_SHAPE };

const shapeOf = (toolName: string): Shape | undefined =>
  Object.hasOwn(TOOL_SHAPES, toolName)
    ? TOOL_SHAPES[toolName]
    : Object.hasOwn(SPEC_SHAPES, toolName)
      ? SPEC_SHAPES[toolName]
      : undefined;

/** The tools whose output shape the kit checks: every built-in tool the SDK gives an output type. */
export const CHECKED_TOOL_OUTPUTS: readonly string[] = Object.keys(TOOL_SHAPES);

/**
 * Fields of a tool's output a hook cannot change: `REPL`'s `refusedCalls`, which "Claude reads
 * unchanged" (code-mode.md, "Redact code output"), so an accepted copy carries the original's.
 */
const KEPT_FIELDS: Readonly<Record<string, readonly string[]>> = { REPL: ['refusedCalls'] };

/** `value` with the fields of `original` a hook cannot change put back. */
export function withKeptFields(toolName: string, value: unknown, original: unknown): unknown {
  const kept = Object.hasOwn(KEPT_FIELDS, toolName) ? KEPT_FIELDS[toolName] : undefined;
  if (kept === undefined || !isRecord(value)) return value;
  const rest = Object.entries(value).filter(([key]) => !kept.includes(key));
  const back = isRecord(original)
    ? kept.filter((key) => Object.hasOwn(original, key)).map((key) => [key, original[key]])
    : [];
  return Object.fromEntries([...rest, ...back]);
}

/** `mcp__<server>__<tool>`: "MCP tool output is passed through without schema validation". */
export const isMcpToolName = (name: string): boolean => name.startsWith('mcp__');

export type ToolOutputCheck =
  | { readonly accepted: true }
  | { readonly accepted: false; readonly message: string };

/** Whether `value` may replace the output of the built-in tool `toolName`. */
export function checkToolOutput(toolName: string, value: unknown): ToolOutputCheck {
  const shape = shapeOf(toolName);
  if (shape === undefined) return { accepted: true };
  const problem = shapeProblem(shape, value);
  return problem === undefined ? { accepted: true } : { accepted: false, message: problem };
}
