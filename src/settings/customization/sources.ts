import type { PolicyHookSource } from '../hooks/origins.ts';

/**
 * Which origins each locked surface still loads: one table per surface, typed over that surface's own
 * origins, from the **Stops** and **Keeps** lists of its sub-key entry in settings-reference.md. A host's
 * discovery (`~/.claude/skills`, `.claude/agents`, `.mcp.json`, `--add-dir`) asks per candidate.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/customization/sources.rs`,
 * docs/RUST.md).
 */
export type SkillsOrigin =
  | 'userSkills'
  | 'projectSkills'
  | 'userCommands'
  | 'projectCommands'
  | 'addDir'
  | 'synced'
  | 'bundled'
  | 'managedDirectory'
  | 'plugin'
  | 'skillsDirPlugin';

export type AgentsOrigin =
  | 'userAgents'
  | 'projectAgents'
  | 'addDir'
  | 'builtIn'
  | 'managedDirectory'
  | 'plugin'
  | 'skillsDirPlugin';

/**
 * `userConfig` is the user-scope servers and `localConfig` the local-scope ones (the default of
 * `claude mcp add`): both live in `~/.claude.json` (mcp.md § Scopes), which the lock stops loading from.
 */
export type McpOrigin =
  | 'userConfig'
  | 'localConfig'
  | 'projectFile'
  | 'managedFile'
  | 'managedSettings'
  | 'plugin'
  | 'skillsDirPlugin';

/** The hooks of every origin the hook policy knows: the same vocabulary, one decision about it. */
export type HooksOrigin = PolicyHookSource;

/** The origins a host names, per surface. */
export interface OriginOf {
  readonly skills: SkillsOrigin;
  readonly agents: AgentsOrigin;
  readonly hooks: HooksOrigin;
  readonly mcp: McpOrigin;
}
