import type { InvocableFrontmatter } from '../format/skill-types.ts';
import type { Command, Skill } from '../format/skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/skills/expand_policy.rs`, docs/RUST.md). */
const call = coreModule('skills/expand-policy');

/**
 * The managed settings that take a skill's `allowed-tools` or `hooks` away (skills.md, "When
 * managed settings lock hooks to plugins" and "When only managed permission rules apply"). The host
 * reads the settings; the kit decides what each does to one skill.
 */
export interface ExpandPolicy {
  /** `strictPluginOnlyCustomization` lists `hooks`. */
  readonly hooksLockedToPlugins?: boolean;
  /** `allowManagedPermissionRulesOnly` is set. */
  readonly managedPermissionRulesOnly?: boolean;
}

/**
 * Where a plugin came from, which the host knows and the kit cannot see. Both of the last two are
 * plugins the managed settings treat like the folders they sit in:
 * - `skills-directory`: declared with a `.claude-plugin` manifest inside `~/.claude/skills/` or
 *   `.claude/skills/` ("hooks from plugins other than skills-directory plugins" keep running; the
 *   `allowManagedPermissionRulesOnly` entry lists them among the sources whose `allowed-tools` are
 *   ignored).
 * - `inline-managed-session`: a plugin loaded for one session only (id ending in `@inline`) in a
 *   Desktop Code or Cowork session whose managed settings set `allowManagedPermissionRulesOnly`, on
 *   Claude Code 2.1.305 or later. The host decides all of those conditions. Any other `@inline`
 *   plugin is `installed`.
 */
export type SkillPluginSource = 'installed' | 'skills-directory' | 'inline-managed-session';

type Source = Pick<Skill | Command, 'origin' | 'context'> & Pick<InvocableFrontmatter, 'hooks'>;

/** Whether the skill's frontmatter hooks register when it is invoked. */
export const hooksRegister = (
  skill: Source,
  policy: ExpandPolicy | undefined,
  source: SkillPluginSource = 'installed',
): boolean => call('hooksRegister', skill, policy, source);

/**
 * Whether the skill's `allowed-tools` grant applies. A skill that declares hooks under the hooks
 * lock "loses its `allowed-tools` pre-approval", except that "when such a skill runs in a subagent
 * through `context: fork`, it keeps the pre-approval". The fork exception is the hooks lock's only;
 * `allowManagedPermissionRulesOnly` ignores the field in a forked skill too.
 */
export const grantApplies = (
  skill: Source,
  policy: ExpandPolicy | undefined,
  source: SkillPluginSource = 'installed',
): boolean => call('grantApplies', skill, policy, source);
