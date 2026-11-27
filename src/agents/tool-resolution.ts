/**
 * The tools a subagent gets, from the definition's `tools` and `disallowedTools`, the pool the main
 * conversation offers and how the run starts.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/tool_resolution.rs`, docs/RUST.md).
 */
import type { AgentDefinition } from '../protocol/official.ts';
import type { HostToolNames } from '../rules/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SubagentRun } from './tool-filters.ts';

const call = coreModule('agents/tool-resolution');

/** The tools the main conversation offers: what a subagent inherits from (a host passes it). */
export interface ToolPool {
  readonly builtin: readonly string[];
  /** `name` is the callable name, `mcp__<server>__<tool>`. */
  readonly mcp: readonly { readonly server: string; readonly name: string }[];
  /**
   * How the names above correspond to Claude Code's, when the host's tools carry other names: the
   * definition's entries match the Claude Code names, the outcome names the host's tools. Absent,
   * the names are Claude Code's.
   */
  readonly names?: HostToolNames;
}

/** An agent as far as its tools go: the two frontmatter fields of the SDK's `AgentDefinition`. */
export type AgentToolFields = Readonly<{
  [K in 'tools' | 'disallowedTools']?: readonly NonNullable<AgentDefinition[K]>[number][];
}> & {
  readonly name: string;
};

/** errors.md, "Agent would be spawned with zero tools": the entries, by what went wrong. */
export interface ToolRefusal {
  readonly message: string;
  /** The entry matches no tool name, usually a typo. */
  readonly unrecognized: readonly string[];
  /** A real tool that subagents cannot use (a background subagent keeps fewer). */
  readonly notAvailableToSubagents: readonly string[];
  /** Valid, but no tool in the session matches it (no such MCP server, `Agent` at the depth limit). */
  readonly matchedNoTools: readonly string[];
}

/** What the `Agent` tool may spawn: a list of types, any type, or none (without the tool). */
export type SpawnTypes = readonly string[] | 'any' | 'none';

export type ToolsOutcome =
  | {
      readonly ok: true;
      readonly tools: readonly string[];
      /** The subagent types the `Agent` tool may spawn; `none` without the tool. */
      readonly spawnTypes: SpawnTypes;
    }
  | { readonly ok: false; readonly refusal: ToolRefusal };

/** `HostToolNames` without `callName`, a function the host keeps: the resolution never reads it. */
const namesData = (names: HostToolNames | undefined) =>
  names === undefined ? undefined : { toClaude: names.toClaude, missing: names.missing };

/**
 * The tools a subagent gets (sub-agents.md, "Available tools" and "Restrict which tools a subagent
 * can use"; tools-reference.md, "Agent tool behavior"), or the refusal Claude Code gives when the
 * `tools` list resolves to nothing (errors.md). Pure over the definition's two fields, the pool the
 * host offers and how the run starts. With `pool.names`, the tools are the host's names for them.
 */
export const resolveSubagentTools = (
  agent: AgentToolFields,
  pool: ToolPool,
  run: SubagentRun,
): ToolsOutcome =>
  call('resolveSubagentTools', agent, { ...pool, names: namesData(pool.names) }, run);
