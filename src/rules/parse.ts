/**
 * Reading a permission rule's text (permissions.md, "Permission rule syntax").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/parse.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { PermissionBehavior } from '../protocol/official.ts';
import type { PermissionRuleValue } from '../protocol/official-tools.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PermissionRule, RuleSource } from './types.ts';

const call = coreModule('rules/parse');

/** Where a rule was written and what it does: the two things the text cannot say. */
export interface RuleOrigin {
  readonly behavior: PermissionBehavior;
  readonly source: RuleSource;
}

export interface RuleParse {
  /** `undefined` when the rule is skipped or ignored; the issues say why. */
  readonly rule: PermissionRule | undefined;
  readonly issues: readonly PluginIssue[];
}

/**
 * Reads one permission rule (permissions.md, "Permission rule syntax"). A rule Claude Code skips or
 * ignores comes back as `rule: undefined` with the issue it reports; nothing throws.
 */
export const parseRule = (text: string, at: RuleOrigin): RuleParse => call('parseRule', text, at);

/** A rule's text, as `parseRule` reads it back to the same value. */
export const formatRule = (value: PermissionRuleValue): string => call('formatRule', value);
