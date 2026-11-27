/**
 * How a host's tool names correspond to Claude Code's, stated once by the host. The docs name only
 * Claude Code's tools, so the kit reads every tool name in those terms: what a subagent's `tools`
 * and `disallowedTools` match, what a hook's `matcher` and `tool_name` see, what a permission rule
 * and a mod's `tool.call` matcher match. Judgment (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/host_tool_names.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/tools/host-tool-names');

export interface HostToolNames {
  /**
   * A host tool name and the Claude Code tool, or tools, it stands for: a built-in (`write_file` to
   * `Write`, `task` to `Agent`, so `Agent(type)` reads it), an MCP tool in `mcp__<server>__<tool>`
   * form, or a list when one host tool does the work of several (`Search` to `['Grep', 'Glob']`).
   * A name left out, or given an empty list, is the same on both sides.
   */
  readonly toClaude?: Readonly<Record<string, string | readonly string[]>>;
  /**
   * The Claude Code name of one call, when it depends on the input (`Search` with a `pattern` is
   * `Grep`, with only a glob `Glob`). Read only when it answers one of the names the tool lists;
   * otherwise, and without it, a call carries the first name listed.
   */
  readonly callName?: (hostTool: string, input: unknown) => string | undefined;
  /** Claude Code tools the host has no tool for: never granted (the restored `Glob`/`Grep`). */
  readonly missing?: readonly string[];
}

/** A `HostToolNames` read both ways. */
export interface ToolNames {
  /** The Claude Code name of a call of a host tool: `callName` over `input`, else the first listed. */
  readonly claudeName: (hostName: string, input?: unknown) => string;
  /** Every Claude Code name a host tool stands for, in the host's order; the name itself when unmapped. */
  readonly claudeNames: (hostName: string) => readonly string[];
  /** The host tools that stand for a Claude Code tool; empty when the host has none. */
  readonly hostNames: (claudeName: string) => readonly string[];
  /** Whether the host has a tool for this Claude Code name. */
  readonly has: (claudeName: string) => boolean;
}

/** The mapping both ways; `undefined` is the host whose tools are Claude Code's. */
export const toolNamesOf = (names: HostToolNames | undefined): ToolNames =>
  call('toolNamesOf', names);
