import type { PermissionBehavior } from '../protocol/official.ts';
import { REMOVED_TOOLS } from '../protocol/tool-names.ts';
import { RESTRICTS } from './restricts.ts';
import { textPatternOf } from './bash/text-pattern.ts';
import type { PermissionRule, RuleSource } from './types.ts';

/**
 * The two startup warnings a rule list can draw, as values (spec 1.1.0 §13.2.1 item 3 and §13.5.2;
 * Judgment J13.33): where the row, the stderr line or the debug-log line goes is the presenting
 * client's, so the kit returns the parts and the caller writes the texts (errors.md, "Matches no
 * tool · check for typos" and "Is not matched by file permission checks"; `fileToolRuleBannerRow`,
 * `fileToolRuleWarning` and `matchesNoToolBannerRow` write them).
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

const removedTools: ReadonlySet<string> = new Set(REMOVED_TOOLS);

/** "Tool names containing `_` or `*` are exempt", and the names of removed tools (permissions.md). */
const exemptName = (name: string, knownTools: ReadonlySet<string>): boolean =>
  name.includes('_') || name.includes('*') || knownTools.has(name) || removedTools.has(name);

function matchesNoTool(rule: PermissionRule, knownTools: ReadonlySet<string>): RuleWarning[] {
  // Only deny and ask rules are checked for a tool they cannot match (permissions.md, "Tool name wildcards").
  if (!RESTRICTS[rule.behavior] || exemptName(rule.value.toolName, knownTools)) return [];
  return [
    { rule: rule.text, behavior: rule.behavior, source: rule.source, kind: 'matches no tool' },
  ];
}

/**
 * The file tools whose path rules file checks never consult, and the rule kind to use instead
 * (errors.md, "Is not matched by file permission checks": `Write`, `NotebookEdit`, `MultiEdit` or
 * `Glob`; Grep is not named there, so it draws no warning).
 */
const NEVER_MATCHED: ReadonlyMap<string, 'Edit' | 'Read'> = new Map([
  ['Write', 'Edit'],
  ['NotebookEdit', 'Edit'],
  ['MultiEdit', 'Edit'],
  ['Glob', 'Read'],
]);

/** "Except in `--allowedTools`, where Claude Code accepts a `Glob` rule without warning" (errors.md). */
const SILENT_SOURCES: { readonly [K in RuleSource['kind']]: boolean } = {
  managed: false,
  'embedder-managed': false,
  project: false,
  local: false,
  user: false,
  'flag-settings': false,
  cli: true,
  session: false,
};

/** The tools whose path rule `--allowedTools` accepts without warning. */
const SILENT_IN_ALLOWED_TOOLS: ReadonlySet<string> = new Set(['Glob']);

const silentlyAccepted = (rule: PermissionRule): boolean =>
  !RESTRICTS[rule.behavior] &&
  SILENT_SOURCES[rule.source.kind] &&
  SILENT_IN_ALLOWED_TOOLS.has(rule.value.toolName);

function neverMatches(rule: PermissionRule): RuleWarning[] {
  const use = NEVER_MATCHED.get(rule.value.toolName);
  const path = textPatternOf(rule);
  // A bare name or `Tool(*)` matches every use and draws no warning (errors.md, "What to do").
  if (use === undefined || path === undefined || path.trim() === '*') return [];
  if (silentlyAccepted(rule)) return [];
  const replacement = `${use}(${path})`;
  const { text, behavior, source } = rule;
  return [{ rule: text, behavior, source, kind: 'never matches', replacement, use }];
}

/** The startup warnings of each rule, in rule order, as values the caller formats and routes. */
export function ruleWarnings(
  rules: readonly PermissionRule[],
  knownTools: ReadonlySet<string>,
): readonly RuleWarning[] {
  return rules.flatMap((rule) => [...matchesNoTool(rule, knownTools), ...neverMatches(rule)]);
}
