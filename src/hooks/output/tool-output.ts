import { shapeProblem } from './tool-shape.ts';
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

/** The tools whose output shape the kit checks: every built-in tool the SDK gives an output type. */
export const CHECKED_TOOL_OUTPUTS: readonly string[] = Object.keys(TOOL_SHAPES);

/** `mcp__<server>__<tool>`: "MCP tool output is passed through without schema validation". */
export const isMcpToolName = (name: string): boolean => name.startsWith('mcp__');

export type ToolOutputCheck =
  | { readonly accepted: true }
  | { readonly accepted: false; readonly message: string };

/** Whether `value` may replace the output of the built-in tool `toolName`. */
export function checkToolOutput(toolName: string, value: unknown): ToolOutputCheck {
  const shape = Object.hasOwn(TOOL_SHAPES, toolName) ? TOOL_SHAPES[toolName] : undefined;
  if (shape === undefined) return { accepted: true };
  const problem = shapeProblem(shape, value);
  return problem === undefined ? { accepted: true } : { accepted: false, message: problem };
}
