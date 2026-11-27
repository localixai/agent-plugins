import type { AgentSource } from '../../format/agent-record.ts';
import type { AgentsOrigin } from '../../settings/customization/sources.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * Where subagents come from and which wins a shared name (sub-agents.md, "Choose the subagent scope",
 * the table "Location, Scope, Priority"): managed settings, the `--agents` flag, `.claude/agents/`,
 * `~/.claude/agents/`, then a plugin's `agents/`. A registry typed over the sources, so a new source
 * fails the typecheck until it says its rank and where its folders are. A built-in ranks below all of
 * them (a user or project subagent named `Explore` "overrides the built-in").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/catalog/scopes.rs`, docs/RUST.md).
 * A rule crosses as its data: where each folder's places are is the core's, which scans them.
 */

const call = coreModule('agents/catalog/scopes');

/** One folder of a source: below each base directory, `.claude/agents` (and `<extra>/agents`). */
export interface AgentFolder {
  /** The name `strictPluginOnlyCustomization` knows it by. */
  readonly origin: AgentsOrigin;
  /** Whether Claude Code watches it for changes ("watches `~/.claude/agents/` and `.claude/agents/`"). */
  readonly watched: boolean;
}

export interface AgentScopeRule {
  /** The Priority column: 1 is the highest. */
  readonly priority: number;
  /** The folders read through the folder-scan port; none for the flag and for plugins. */
  readonly folders: readonly AgentFolder[];
}

export const AGENT_SCOPE_RULES: { readonly [S in AgentSource]: AgentScopeRule } =
  call('AGENT_SCOPE_RULES');
