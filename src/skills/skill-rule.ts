import type { Command, Skill } from '../format/skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Every name a skill goes by, for the `Skill(...)` rules that name it (skills.md, "Restrict
 * Claude's skill access", the table of what a deny rule blocks beyond the name you write). The rule
 * matcher itself is the core's (`crates/agent-plugins/src/rules/target/skill.rs`); it reads these
 * names from `PreparedCall.skillNames`.
 * Only a deny rule sees the names beyond the invoked one ("Claude Code matches an allow rule only
 * against the skill's own name and the name in Claude's invocation"), so the host passes the list
 * to the matcher and the matcher decides who may read it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/skill_rule.rs`, docs/RUST.md).
 */
const call = coreModule('skills/skill-rule');

type Named = Pick<Skill | Command, 'name' | 'command' | 'origin'>;

/**
 * The skill's command first, then every other name, then the aliases the host knows (a bundled
 * skill's alias, `/review` for `/code-review`: the kit holds no catalog of bundled skills). Each
 * name once, in that order.
 */
export const skillNamesOf = (skill: Named, aliases: readonly string[] = []): readonly string[] =>
  call('skillNamesOf', skill, aliases);
