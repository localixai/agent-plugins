import type { SessionSkill } from './session-skill-catalog.ts';
import type { SkillSource } from './session-skill-entries.ts';
import type { ScanScope } from '../skills/index.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What a host reads off a `SessionSkill` row for its own list: the plugin it belongs to, the scope
 * it loaded from, and the frontmatter `keywords` some skills carry. Pure readers of the row.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_skill_readers.rs`, docs/RUST.md).
 */

const call = coreModule('host/session-skill-readers');

/** A plugin skill's plugin: the id the host uses for it and the name its commands are scoped by. */
export interface SkillPlugin {
  readonly id: string;
  readonly name: string;
}

/** The plugin a skill or command row belongs to; `undefined` for a folder or host skill. */
export const skillPlugin = (skill: SessionSkill): SkillPlugin | undefined =>
  call('skillPlugin', skill);

/** Where a row loaded from: a folder scope (skills.md, "Choose where skills load"), `plugin`, or `host`. */
export type SkillScope = ScanScope | Exclude<SkillSource['kind'], 'folder'>;

export const skillScope = (skill: SessionSkill): SkillScope => call('skillScope', skill);

/**
 * The frontmatter `keywords` of a row: a YAML list keeps its strings, a string is split at commas;
 * anything else is none. `keywords` is no field of the skills frontmatter table, and "Claude Code
 * ignores a field it doesn't recognize without reporting an error" (skills.md, "Frontmatter
 * reference"): it changes nothing the kit or Claude Code does with the skill. Judgment
 * (spec/judgments.md, `skillKeywords`): the reader is a host's own use of a key it keeps.
 */
export const skillKeywords = (skill: SessionSkill): readonly string[] =>
  call('skillKeywords', skill);
