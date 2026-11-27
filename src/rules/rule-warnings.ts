import type { PermissionBehavior } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PermissionRule, RuleSource } from './types.ts';

const call = coreModule('rules/rule-warnings');

/**
 * The two startup warnings a rule list can draw, as values (spec 1.1.0 §13.2.1 item 3 and §13.5.2;
 * Judgment J13.33): where the row, the stderr line or the debug-log line goes is the presenting
 * client's, so the kit returns the parts and the caller writes the texts (errors.md, "Matches no
 * tool · check for typos" and "Is not matched by file permission checks"; `fileToolRuleBannerRow`,
 * `fileToolRuleWarning` and `matchesNoToolBannerRow` write them).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/rule_warnings.rs`, docs/RUST.md).
 */
export type RuleWarningKind = 'matches no tool' | 'never matches';

export interface RuleWarning {
  /** The rule as written, trimmed. */
  readonly rule: string;
  readonly behavior: PermissionBehavior;
  readonly source: RuleSource;
  readonly kind: RuleWarningKind;
  /** `never matches` only: the rule to write instead, `Edit(<path>)`, or `Read(<path>)` for `Glob`. */
  readonly replacement?: string;
  /** `never matches` only: `Edit` or `Read`, the rule kind file checks consult. */
  readonly use?: 'Edit' | 'Read';
}

/** The startup warnings of each rule, in rule order, as values the caller formats and routes. */
export const ruleWarnings = (
  rules: readonly PermissionRule[],
  knownTools: ReadonlySet<string>,
): readonly RuleWarning[] => call('ruleWarnings', rules, knownTools);
