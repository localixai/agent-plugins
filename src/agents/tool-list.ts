/**
 * One entry of the `tools` or `disallowedTools` field (sub-agents.md, "Restrict which tools a
 * subagent can use"): a tool name, with the specifier it was written with, or `Agent(type, ...)`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/tool_list.rs`, docs/RUST.md).
 */
import type { PermissionBehavior } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('agents/tool-list');

export interface ToolEntry {
  /** The tool name; `Task` (the pre-2.1.63 name, "still work as aliases") reads as `Agent`. */
  readonly name: string;
  /** The text in parentheses (`git push *` of `Bash(git push *)`); not for `Agent`. */
  readonly specifier?: string;
  /**
   * Only for `Agent`: the subagent types it may spawn, or `any`. "Restrict which subagents can be
   * spawned": `Agent(worker, researcher)` is an allowlist, `Agent` alone allows any.
   */
  readonly spawn?: readonly string[] | 'any';
}

export const AGENT_TOOL: 'Agent' = call('AGENT_TOOL');

/** Whether a tool name is the Agent tool: `Task` (its name before 2.1.63) "still work[s] as an alias". */
export const isAgentToolName = (name: string): boolean => call('isAgentToolName', name);

/** Reads one entry of the field, as `readAgent` keeps it (a comma inside parentheses is not a split). */
export const parseToolEntry = (entry: string): ToolEntry => call('parseToolEntry', entry);

/**
 * Whether `entry` names the tool `toolName`. The behaviour is the one permissions.md gives the same
 * name in a rule: `tools` reads like an allow rule and `disallowedTools` like a deny rule
 * (spec/judgments.md). So `mcp__reports__v2` never matches in `tools` and does in `disallowedTools`,
 * and an unanchored glob such as `mcp__*` matches only as a deny entry. A specifier never narrows
 * the match: "still removes the whole tool".
 */
export const toolEntryMatches = (
  entry: ToolEntry,
  toolName: string,
  behavior: PermissionBehavior,
): boolean => call('toolEntryMatches', entry, toolName, behavior);
