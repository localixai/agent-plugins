/**
 * A handler's `if` against the tool call (hooks.md, "Common fields"): read as permission rule
 * syntax on a tool event, never run on another, and run with a warning when no evaluator is given.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/if_guard.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { IfResult } from '../../rules/index.ts';
import type { MatchContext } from '../../rules/types.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { MonitorTreatment, RegisteredHandler, ToolCallView } from '../contracts.ts';

const call = coreModule('hooks/select/if-guard');

/** Evaluates one `if` rule against a tool call, with whatever context rules need (cwd, home). */
export type IfEvaluator = (rule: string, toolName: string, toolInput: unknown) => IfResult;

/** The evaluator of permission rule syntax over a match context the host (or the engine) built. */
export const ifEvaluatorFor = (context: MatchContext): IfEvaluator =>
  call('ifEvaluatorFor', context);

export interface GuardResult {
  readonly runs: boolean;
  readonly issues: readonly PluginIssue[];
}

/** What the `if` is read against: the tool call (absent on a non-tool event), how a Monitor call got here, the rule reader. */
export interface GuardSubject {
  readonly call: ToolCallView | undefined;
  readonly monitor: MonitorTreatment | undefined;
  readonly evaluate: IfEvaluator | undefined;
}

/**
 * hooks.md, "Common fields": `if` is "only evaluated on tool events ... On other events, a hook with
 * `if` set never runs". On a tool event the rule is read as permission rule syntax; a call the rule
 * cannot be decided for runs the hook ("it runs your hook regardless of the pattern"). A Monitor
 * call that reached the hook through a `Bash` matcher is a Bash call to `if`: "In `if`, name Bash
 * ... The hook doesn't run if its `if` names Monitor". A `)` followed by `&&` or `||` and another
 * rule is still read as one rule, with a warning that it never fires (spec/judgments.md).
 */
export const guardWithIf = (
  entry: RegisteredHandler,
  subject: GuardSubject,
  path: string,
): GuardResult => call('guardWithIf', entry, subject, path);
