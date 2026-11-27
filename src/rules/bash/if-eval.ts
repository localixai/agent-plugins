/**
 * A hook's `if` evaluated against a tool call (hooks.md, "Common fields").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/if_eval.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { MatchContext } from '../types.ts';

const call = coreModule('rules/bash/if-eval');

/** Whether a hook's `if` lets its handler run, with what the kit found wrong in the rule. */
export interface IfResult {
  readonly runs: boolean;
  readonly issues: readonly PluginIssue[];
}

/**
 * Whether a hook handler with this `if` runs for this tool call (hooks.md, "Common fields"): the
 * rule is read as permission rule syntax and matches the way a deny or ask rule does, any
 * subcommand being enough, except that a single-directory path pattern keeps the depth of an allow
 * rule (`depthOf`). A rule the kit cannot read runs the handler and reports why: the `if`
 * filter is best effort (spec/judgments.md). Which events evaluate `if` is the hooks engine's.
 */
export const evaluateIf = (
  ifRule: string,
  toolName: string,
  toolInput: unknown,
  ctx: MatchContext,
): IfResult => call('evaluateIf', ifRule, toolName, toolInput, ctx);
