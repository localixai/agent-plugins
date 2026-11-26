/**
 * The skills a subagent preloads into its context at startup (sub-agents.md, "Preload skills into
 * subagents"). The kit plans; loading skill content and building the system prompt are the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/preload_skills.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('agents/preload-skills');

/** What the host knows about its skills; narrow so the skills package need not be imported. */
export interface SkillCatalog {
  /** The skill exists and is enabled (a missing one and one the organization disabled are alike). */
  has(name: string): boolean;
  /** Claude can invoke it: not `disable-model-invocation: true` ("the same set of skills"). */
  isModelInvocable(name: string): boolean;
}

export type SkillSkipReason = 'missing' | 'not-model-invocable' | 'beyond-limit';

export interface PreloadPlan {
  /** The skills whose full content is injected at startup, in the list's order. */
  readonly load: readonly string[];
  /** Listed skills that are not injected; Claude Code "skips it and logs a warning". */
  readonly skipped: readonly { readonly name: string; readonly reason: SkillSkipReason }[];
}

/**
 * The skills a subagent preloads from its `skills` field. Only the first 32 distinct names count:
 * a name past them is skipped, and so is a listed name that is missing or not model-invocable (that
 * one still uses up its place among the 32, as the docs count names "in the list"). A repeated name
 * is read once and not reported (judgment). Which skills the subagent may invoke through the Skill
 * tool is the `tools` field's, not this one's.
 */
export const planPreloadedSkills = (
  names: readonly string[] | undefined,
  catalog: SkillCatalog,
): PreloadPlan => call('planPreloadedSkills', names, catalog);
