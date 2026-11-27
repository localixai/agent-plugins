import type { PluginIssue } from '../../format/issues.ts';
import type { PermissionRule } from '../../rules/types.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ScannedAgent } from './scan.ts';
import type { CatalogAgent } from './types.ts';

/**
 * Which definition a name means (sub-agents.md, "Choose the subagent scope": "When multiple subagents
 * share the same name, Claude Code uses the one from the higher-priority location"; "the definition
 * closest to the working directory" among nested project directories; a user or project subagent
 * named `Explore` overrides the built-in) and which subagents the settings switch off ("Disable
 * specific subagents": `Agent(name)` deny rules, or the `Agent` tool itself).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/catalog/resolve.rs`, docs/RUST.md).
 */

const call = coreModule('agents/catalog/resolve');

export interface AgentSources {
  readonly builtin?: readonly CatalogAgent[];
  /** The `--agents` flag's agents. */
  readonly flag?: readonly CatalogAgent[];
  /** The files of the managed, project, user and added folders, as `scanAgents` read them. */
  readonly scanned?: readonly ScannedAgent[];
  /** The agents of the enabled plugins, under their scoped names. */
  readonly plugin?: readonly CatalogAgent[];
  /** The permission rules of the session; only deny rules matter here (settings and `--disallowedTools`). */
  readonly denied?: readonly PermissionRule[];
}

export interface Shadowed {
  readonly agent: CatalogAgent;
  /** The definition that took the name. */
  readonly by: CatalogAgent;
}

export interface AgentCatalog {
  /** The subagents a session can run, highest priority first. */
  readonly agents: readonly CatalogAgent[];
  /** Removed by a deny rule: the built-in or custom subagents that were defined. */
  readonly disabled: readonly CatalogAgent[];
  /** Definitions that lost their name to a higher-priority one. */
  readonly shadowed: readonly Shadowed[];
  /** Two files of one agents directory with one name. */
  readonly issues: readonly PluginIssue[];
  /** False when a deny rule names the `Agent` tool itself: no subagent can be spawned. */
  readonly delegation: boolean;
}

/** The agents of every source, one definition per name, minus what a deny rule switched off. */
export const resolveAgents = (sources: AgentSources = {}): AgentCatalog =>
  call('resolveAgents', sources);

export type SubagentTypeOutcome =
  | { readonly ok: true; readonly agent: CatalogAgent }
  | { readonly ok: false; readonly message: string };

/**
 * The subagent an Agent tool call runs (sub-agents.md, "Built-in subagents": "An Agent tool call that
 * omits `subagent_type` fails with `subagent_type is required` when the session has no
 * `general-purpose` subagent to fall back on"; errors.md, "subagent_type is required"). The message
 * lists the subagents the session has so Claude can retry with one of them. Judgment (spec/judgments.md):
 * the list is alphabetical, and a type the session does not have, or one a deny rule switched off, gets
 * the same list under a message of the kit's own.
 */
export const resolveSubagentType = (
  catalog: AgentCatalog,
  subagentType: string | undefined,
): SubagentTypeOutcome => call('resolveSubagentType', catalog, subagentType);
