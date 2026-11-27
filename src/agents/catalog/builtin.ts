import { coreModule } from '../../wasm/bridge.ts';
import type { CatalogAgent } from './types.ts';

/**
 * The built-in subagents (sub-agents.md, "Built-in subagents"): the three the page gives a tab each and
 * the "Other" table. A registry typed over the names, so a built-in added to the list fails the
 * typecheck until it says what it is. The docs publish no prompt (a built-in's is Claude Code's own),
 * so `prompt` is empty; a description is the page's own sentence for it (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/catalog/builtin.rs`, docs/RUST.md).
 */
const call = coreModule('agents/catalog/builtin');

export const BUILTIN_AGENT_NAMES: readonly [
  'Explore',
  'Plan',
  'general-purpose',
  'claude',
  'statusline-setup',
  'claude-code-guide',
] = call('BUILTIN_AGENT_NAMES');

/** What switches the built-ins off. */
export interface BuiltinPolicy {
  /** False with `-p` or through the Agent SDK. */
  readonly interactive: boolean;
  /** `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS=1`. */
  readonly disableExplorePlan: boolean;
  /** `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS=1`; it acts in non-interactive mode only. */
  readonly disableAll: boolean;
}

/** The built-in subagents a session registers, in the page's order. */
export const builtinAgents = (policy: BuiltinPolicy): readonly CatalogAgent[] =>
  call('builtinAgents', policy);
