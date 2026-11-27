import { coreModule } from '../../wasm/bridge.ts';

/**
 * A core package's skill (core §7.1): its `SKILL.md` conforms to the Agent Skills specification
 * (agentskills.io/specification, "Frontmatter"), or the client skips the skill. The frontmatter is
 * required; `name` is 1–64 characters of `a-z`, `0-9` and `-`, no hyphen at either end, no `--`,
 * and the parent directory's name; `description` is 1–1024 characters; `license` is a string,
 * `compatibility` 1–500 characters, `metadata` a map of strings, `allowed-tools` a space-separated
 * string.
 *
 * Judgments (spec/judgments.md): a field the Agent Skills specification does not define is a
 * warning, not a skip, since the specification lists its fields without forbidding others (Agent
 * Plugins Extended §18.3.2 lets them exist); a description of blanks only is empty.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/skill.rs`, docs/RUST.md).
 */

const call = coreModule('format/core/skill');

export interface CoreSkillCheck {
  /** Why the skill is skipped; empty when it conforms. */
  readonly problems: readonly string[];
  /** Fields outside the Agent Skills specification, in the order they appear. */
  readonly unknownFields: readonly string[];
}

/** Checks a `SKILL.md` found in `skills/<directory>/` by the Agent Skills specification. */
export const checkCoreSkill = (text: string, directory: string): CoreSkillCheck =>
  call('checkCoreSkill', text, directory);
