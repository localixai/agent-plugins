/**
 * A hook's `tool_input` read against its tool's table.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/inputs/parse_input.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/inputs/parse-input');

export interface InputParse {
  /** The input as sent, a copy; `{}` when it was not an object. */
  readonly input: Record<string, unknown>;
  /** Each problem, at `tool_input.<field>` (`tool_input` for the input itself). */
  readonly issues: readonly PluginIssue[];
}

/**
 * Reads a hook's `tool_input` against the table of its tool (hooks.md, "PreToolUse input"): a wrong
 * type or a missing required field is an issue and nothing throws. The input comes back whole, so a
 * hook can still read what the table does not list. A tool without a table (an MCP tool, `Skill`) is
 * returned as it is (spec/judgments.md).
 */
export const parseToolInput = (tool: string, raw: unknown): InputParse =>
  call('parseToolInput', tool, raw);
