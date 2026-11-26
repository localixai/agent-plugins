/**
 * The arguments of an `mcp_tool` hook (hooks.md, "MCP tool hook fields"): `input` holds "arguments
 * passed to the tool. String values support `${path}` substitution from the hook's JSON input,
 * such as `"${tool_input.file_path}"`".
 *
 * The page names only that form, so the rest is the kit's reading (spec/judgments.md): a path is
 * keys joined by dots, and a number is an array index; strings nested in objects and arrays are
 * substituted too; the text a path brings in is not read again; a value that is not a string is
 * written as text (an object as JSON); a path with no value is an empty string and a warning.
 * Numbers, booleans and null stay as they are, and so do keys.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/mcp_input.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('hooks/handlers/remote/mcp-input');

/** The arguments to send, and a warning for each path that had no value. */
export interface McpArguments {
  readonly input: Record<string, unknown>;
  readonly issues: readonly PluginIssue[];
}

/** `input` with every string's references read from `hookInputJson`, the input the hook receives. */
export const substituteInput = (
  input: Readonly<Record<string, unknown>> | undefined,
  hookInputJson: string,
  issuePath: string,
): McpArguments => call('substituteInput', input, hookInputJson, issuePath);
