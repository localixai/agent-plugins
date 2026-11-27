import { coreModule } from '../../wasm/bridge.ts';
import type { SpecifierMatcher } from '../matchers.ts';

/**
 * `Skill(name)` and `Skill(name *)` (skills.md, "Restrict Claude's skill access"): `Skill(name)` is
 * an exact match and `Skill(name *)` a prefix match with any arguments. The prefix is a whole name
 * or a namespace: `Skill(anthropic-skills *)` covers `anthropic-skills:pdf` and `Skill(anthropic *)`
 * does not, "because a prefix outside the namespace doesn't match the names inside it". A deny rule
 * also sees every other name the host reports in `PreparedCall.skillNames`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/target/skill.rs`, docs/RUST.md).
 */
const call = coreModule('rules/target/skill');

export const matchSkillRule: SpecifierMatcher = (rule, prepared, ctx) =>
  call('matchSkillRule', rule, prepared, ctx);
