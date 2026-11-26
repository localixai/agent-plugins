import type { PermissionMode } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/tool_filters.rs`, docs/RUST.md).
 */
const call = coreModule('agents/tool-filters');

/** How a subagent is started, as far as its tools are concerned. */
export interface SubagentRun {
  /** A fork "skips both filters and receives the main conversation's exact tool pool". */
  readonly kind: 'subagent' | 'fork';
  /** Runs in the background: the second filter applies. */
  readonly background: boolean;
  /** At the depth limit Claude Code withholds `Agent` from every subagent. */
  readonly atDepthLimit: boolean;
  /** The subagent's own `permissionMode`: `ExitPlanMode` stays only for `plan`. */
  readonly permissionMode: PermissionMode;
  /** The agent runs as the main thread (`claude --agent` or the `agent` setting). */
  readonly asMainThread: boolean;
  /**
   * The host gives this subagent `SubagentHandback` (tools-reference.md: auto mode, a local
   * subagent that is not a fork). It is granted whatever `tools` and `disallowedTools` say.
   */
  readonly handback?: boolean;
  /**
   * The subagent is an agent team teammate (sub-agents.md, "Available tools"): in the background
   * it also keeps the task tools and cron tools.
   */
  readonly teammate?: boolean;
  /**
   * The subagent already runs in its own working directory (`isolation: worktree`): tools-reference.md
   * says `ExitWorktree` is "Not available to subagents that already run in their own working directory".
   */
  readonly ownWorkingDirectory?: boolean;
}

/** The built-in tools a background subagent keeps (the second filter); every MCP tool stays. */
export const BACKGROUND_BUILTIN_TOOLS: readonly [
  'Read',
  'Grep',
  'Glob',
  'LSP',
  'Bash',
  'PowerShell',
  'Edit',
  'Write',
  'NotebookEdit',
  'WebFetch',
  'WebSearch',
  'TodoWrite',
  'Skill',
  'ToolSearch',
  'EnterWorktree',
  'ExitWorktree',
  'Monitor',
  'TaskStop',
  'SendMessage',
  'Artifact',
] = call('BACKGROUND_BUILTIN_TOOLS');

/**
 * Whether a subagent of this run never gets the built-in tool `tool`: the first filter, then (for
 * a background run) the second. A fork and an agent running as the main thread are not narrowed
 * (spec/judgments.md).
 */
export const removedFromSubagent = (tool: string, run: SubagentRun): boolean =>
  call('removedFromSubagent', tool, run);
