import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { Skill, SkillLayout } from './skill-types.ts';

/**
 * One `SKILL.md` read into a `Skill`: its frontmatter, its name and command, and where it is.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/skill.rs`, docs/RUST.md).
 */

const call = coreModule('format/skill');

export interface SkillRead {
  readonly skill: Skill;
  readonly issues: readonly PluginIssue[];
}

/** `skills/review/SKILL.md` is in `skills/review`; a file at the root is in `.`. */
export const directoryOf = (path: string): string => call('directoryOf', path);

/**
 * Read one `SKILL.md`. Never throws and never refuses the file (skills.md: "If the YAML between
 * the markers doesn't parse, the skill still loads with no fields set"; an unknown field is
 * ignored "without reporting an error"). What the docs leave open is in spec/judgments.md: a YAML
 * error and a value of the wrong shape are warnings naming the file.
 *
 * The name and the command come from `resolveSkillName`; its issues are reported against the file
 * (an error means the skill is refused: a name Claude Code does not load).
 */
export const readSkill = (path: string, text: string, layout: SkillLayout): SkillRead =>
  call('readSkill', path, text, layout);
