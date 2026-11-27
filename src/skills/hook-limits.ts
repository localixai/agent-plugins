import type { SkillOrigin } from '../format/skill-types.ts';
import type { HooksSettings } from '../protocol/hook-config.ts';
import { coreModule } from '../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/skills/hook_limits.rs`, docs/RUST.md). */
const call = coreModule('skills/hook-limits');

/**
 * The limits on the hooks a skill registers (hooks.md, "Limits on skill hooks"): per source of
 * skills and per session, at most 1,024 hooks and 16,384 characters of matchers, built up skill by
 * skill. Pure counting: whether a session keeps the totals, and registering and firing the hooks,
 * are the host's and the hooks engine's.
 */
export const SKILL_HOOK_LIMITS: { readonly hooks: 1024; readonly matcherChars: 16384 } =
  call('SKILL_HOOK_LIMITS');

export interface HookUsage {
  readonly hooks: number;
  readonly matcherChars: number;
}

export const NO_HOOKS_YET: HookUsage = call('NO_HOOKS_YET');

/**
 * The hooks and matcher characters of one skill's `hooks`: "A matcher with several hooks under it
 * counts once for each hook". Judgment (spec/judgments.md): characters are code points, and a group
 * with no matcher adds none.
 */
export const countSkillHooks = (hooks: HooksSettings): HookUsage => call('countSkillHooks', hooks);

export interface HooksAdmission {
  readonly admitted: boolean;
  /** The source's totals after this skill: unchanged when it was refused. */
  readonly used: HookUsage;
}

/**
 * Whether a skill's hooks fit what its source already registered. A skill that does not fit
 * "registers none of its hooks", so the totals do not move and a later, smaller skill may still fit.
 */
export const admitSkillHooks = (used: HookUsage, hooks: HooksSettings): HooksAdmission =>
  call('admitSkillHooks', used, hooks);

/**
 * The source a skill counts under: the project, the personal folder, or one plugin. Judgment: the
 * docs name no limit for a skill synced from claude.ai or a bundled skill, so each counts as a
 * source of its own.
 */
export const hookSourceOf = (origin: SkillOrigin): string => call('hookSourceOf', origin);
