/**
 * The output shape of the built-in tools a `PostToolUse` hook may replace (hooks.md, "PostToolUse
 * decision control"): "For built-in tools, a value that doesn't match the tool's output schema is
 * ignored and the original output is used. MCP tool output is passed through without schema
 * validation." Agent Plugins Extended J7.138: the shape is the tool's output type in the SDK's
 * `sdk-tools.d.ts`, where one is defined; a tool without one passes unchecked. The shapes are read
 * from that file (the core's `tool_shapes/`, written by `scripts/tool-output-shapes.ts`), every
 * nested field included; a field the type does not declare is allowed, as the types are open.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/tool_output.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/tool-output');

/** The tools whose output shape the kit checks: every built-in tool the SDK gives an output type. */
export const CHECKED_TOOL_OUTPUTS: readonly string[] = call('CHECKED_TOOL_OUTPUTS');

/** `mcp__<server>__<tool>`: "MCP tool output is passed through without schema validation". */
export const isMcpToolName = (name: string): boolean => call('isMcpToolName', name);

export type ToolOutputCheck =
  | { readonly accepted: true }
  | { readonly accepted: false; readonly message: string };

/** Whether `value` may replace the output of the built-in tool `toolName`. */
export const checkToolOutput = (toolName: string, value: unknown): ToolOutputCheck =>
  call('checkToolOutput', toolName, value);
