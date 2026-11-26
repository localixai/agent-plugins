import type { AgentRecord, AgentSource } from '../format/agent-record.ts';
import type { HookMatcherGroup, HooksSettings } from '../protocol/hook-config.ts';
import type { HookEvent } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The hooks an agent's frontmatter defines, and which of them fire (sub-agents.md, "Hooks in
 * subagent frontmatter"). The kit selects; running them is the hooks engine's. It returns only the
 * agent's own groups: "in the main-session case they run alongside any hooks defined in
 * `settings.json`", so the host merges them with the settings' groups.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/hooks.rs`, docs/RUST.md).
 */
const call = coreModule('agents/hooks');

/** How the agent runs: "spawned as a subagent" or "as the main session via `--agent`". */
export type AgentRunKind = 'subagent' | 'main-session';

type HookSource = Pick<AgentRecord, 'hooks' | 'source'>;

/**
 * What the agent's hooks are for this run. A plugin agent has none: Claude Code ignores `hooks`
 * "for security reasons" (the reader already drops them, and this holds for a record built another
 * way). The record is not changed.
 */
export const agentHookGroups = (agent: HookSource, run: AgentRunKind): HooksSettings =>
  call('agentHookGroups', agent, run);

/**
 * "Until you trust the folder, the subagent still runs, but Claude Code skips its frontmatter
 * hooks." `folderTrusted` is the host's one answer to "did the user accept the trust dialog for
 * this very folder": a parent folder's trust and a `-p` session do not make it true, which is why
 * the kit does not compute it.
 */
export const hooksTrustGate = (
  source: AgentSource,
  folderTrusted: boolean,
): 'run' | 'skip-untrusted' => call('hooksTrustGate', source, folderTrusted);

export interface AgentHookSelection {
  /** The groups to run, in the order the file lists them. */
  readonly groups: readonly HookMatcherGroup[];
  /** Why an agent that has hooks selected none: not active, or its folder is not trusted. */
  readonly skipped?: 'untrusted' | 'inactive';
}

export interface AgentHookContext {
  readonly run: AgentRunKind;
  /** The event that occurred; for a subagent's `Stop`, `SubagentStop`. */
  readonly event: HookEvent;
  /**
   * The event's matcher input (the tool name, the agent type, ...); none for `Stop`. A Monitor
   * call is passed as `Monitor`; the kit adds the `Bash` reading where the docs give it.
   */
  readonly matcherValue: string | undefined;
  /** "These hooks only run while that specific subagent is active." */
  readonly active: boolean;
  /** See `hooksTrustGate`. */
  readonly folderTrusted: boolean;
}

/**
 * The groups of the agent's frontmatter that fire for one occurrence of an event. Judgments: an
 * inactive agent is reported before an untrusted one, and a skip is reported only for an agent
 * that has hooks, so the host logs the trust error only where hooks were lost.
 */
export const selectAgentHooks = (agent: HookSource, ctx: AgentHookContext): AgentHookSelection =>
  call('selectAgentHooks', agent, ctx);
