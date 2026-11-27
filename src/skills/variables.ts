import type { PluginIssue } from '../format/issues.ts';
import type { Command, Skill } from '../format/skill-types.ts';
import type {
  ResolvedUserConfig,
  ShellPaths,
  SubstitutionScope,
} from '../loading/resolve/index.ts';
import type { EffortLevel } from '../protocol/official.ts';
import type { Environment, Locations } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The `${CLAUDE_*}` variables of a skill's text (skills.md, "Available string substitutions").
 * The mechanism is `loading/resolve` (the one home of variable substitution, docs/ARCHITECTURE.md);
 * this file decides only what a skill adds to it: its own three variables (`CLAUDE_SESSION_ID`,
 * `CLAUDE_EFFORT`, `CLAUDE_SKILL_DIR`) and which of the six an origin or a session leaves as
 * written.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/variables.rs`, docs/RUST.md).
 */

const call = coreModule('skills/variables');

/** The ports a skill's variables are read through. */
export interface SkillVariablePorts {
  readonly locations: Locations;
  readonly environment: Environment;
}

export interface SkillSession {
  /** `${CLAUDE_SESSION_ID}`. */
  readonly sessionId: string;
  /**
   * `${CLAUDE_EFFORT}`: the effort level in force while the skill runs. A skill that sets its own
   * `effort` runs at that level, so pass the one `resolveTurnOverrides` chose. Absent, the
   * placeholder stays as written.
   */
  readonly effort?: EffortLevel;
  /** The id `Locations` knows the plugin by; default the plugin name of the skill's origin. */
  readonly pluginId?: string;
  /** The saved `userConfig` values of the plugin, for `${user_config.KEY}` in a plugin skill. */
  readonly userConfig?: ResolvedUserConfig;
  /** A Windows host: paths are written with forward slashes (docs/API.md, "Variable substitution"). */
  readonly windows?: boolean;
  /**
   * The session runs in an isolated container, a cloud or Cowork session, where a synced skill
   * "keeps the behavior a local skill has" (skills.md, "How Claude Code handles the body of a
   * synced skill"). False is a session on the owner's machine.
   */
  readonly isolatedSession?: boolean;
  /**
   * The absolute folder `skill.dir` is relative to, when it is not the origin's usual root (a
   * project skill found in a nested `.claude/skills` or in an added directory).
   */
  readonly skillRoot?: string;
  /**
   * How a `!` command writes the path variables for the shell (`SubstitutionScope.shellPaths`, the
   * host's `shellPaths`): default the reference, with the value in the command's environment.
   */
  readonly shellPaths?: ShellPaths;
  readonly signal?: AbortSignal;
}

/** What `substituteVariables` needs: the shared scope and the names this skill leaves as written. */
export interface SkillVariables {
  readonly scope: SubstitutionScope;
  /** Variable names (without `${}`) that stay as written for this origin and session. */
  readonly literal: readonly string[];
  /** Only a plugin skill is told about an unresolved `${user_config.KEY}`. */
  readonly reportsIssues: boolean;
}

/**
 * A skill's path as the host writes it: `path` as is when it is absolute, `root` for `.`, else
 * `path` below `root`. The one rule for `${CLAUDE_SKILL_DIR}` and for where a session's skill lives
 * (`SessionSkill.file`, `SessionSkill.directory`).
 */
export const skillPathIn = (
  root: string,
  path: string,
  locations: Pick<Locations, 'joinPath'>,
): string => call('skillPathIn', root, path, locations);

/** Anything with an origin; a `Command` has no folder of its own. */
export type VariableSource = Pick<Skill | Command, 'origin'> & { readonly dir?: string };

/** The scope `loading/resolve` substitutes in, and the names this skill leaves as written. */
export const buildVariables = (
  source: VariableSource,
  session: SkillSession,
  ports: SkillVariablePorts,
): SkillVariables =>
  call('buildVariables', { origin: source.origin, dir: source.dir }, session, {
    locations: ports.locations,
    environment: ports.environment,
  });

export interface VariablesSubstituted {
  readonly text: string;
  /** Warnings from the shared substitution, for a plugin skill only. */
  readonly issues: readonly PluginIssue[];
}

/**
 * `${CLAUDE_*}` in skill content: the paths written in. Used for the skill's Markdown, the text a
 * `!` command's permission check reads, and the Bash rules of `allowed-tools`
 * (`substituteAllowedTools`).
 */
export const substituteVariables = (
  text: string,
  vars: SkillVariables,
  path = '',
): Promise<VariablesSubstituted> => call.async('substituteVariables', text, vars, path);

export interface AllowedToolsSubstituted {
  /** The rules of `allowed-tools`, in order, the Bash ones with their `${CLAUDE_*}` replaced. */
  readonly rules: readonly string[];
  readonly issues: readonly PluginIssue[];
}

/**
 * The `allowed-tools` of a skill with `${CLAUDE_*}` replaced in its Bash rules (skills.md: the rule
 * "then matches the exact command the skill body tells Claude to run"). Any other rule is kept as
 * written. The host passes the result on to the rule matcher.
 */
export const substituteAllowedTools = (
  skill: Pick<Skill, 'allowedTools'>,
  vars: SkillVariables,
  path = 'allowed-tools',
): Promise<AllowedToolsSubstituted> =>
  call.async('substituteAllowedTools', { allowedTools: skill.allowedTools }, vars, path);
