import { warning, type PluginIssue } from '../format/issues.ts';
import { fileToolRuleWarning } from '../format/host-errors.ts';
import { SETTINGS_FILE } from '../format/settings-files.ts';
import { wildcardBeforeSubcommand } from './bash/wildcard-warning.ts';
import { ruleWarnings, type RuleWarning, type RuleWarningKind } from './rule-warnings.ts';
import type { PermissionRule, RuleSource } from './types.ts';

/**
 * What the parentheses of the "is not matched by file permission checks" warning name: "a settings
 * file path, or the flag itself for `--allowed-tools` and `--disallowed-tools`", and for managed
 * settings "the full warning's parentheses read `managed policy settings`" (errors.md).
 */
const FILE_TOOL_SOURCE: {
  readonly [K in RuleSource['kind']]: (
    warned: RuleWarning,
    source: Extract<RuleSource, { kind: K }>,
  ) => string;
} = {
  managed: () => 'managed policy settings',
  'embedder-managed': () => 'managed policy settings',
  project: () => SETTINGS_FILE.project,
  local: () => SETTINGS_FILE.local,
  user: () => SETTINGS_FILE.user,
  'flag-settings': (_warned, source) => source.file,
  cli: (warned) => (warned.behavior === 'allow' ? '--allowed-tools' : '--disallowed-tools'),
  session: () => 'session',
};

const sourceOf = <K extends RuleSource['kind']>(
  warned: RuleWarning & { readonly source: Extract<RuleSource, { kind: K }> },
): string => FILE_TOOL_SOURCE[warned.source.kind](warned, warned.source);

/** The stderr form of each warning kind (errors.md, the non-interactive text run). */
const STDERR: { readonly [K in RuleWarningKind]: (warned: RuleWarning) => string } = {
  'matches no tool': (warned) =>
    `Permission ${warned.behavior} rule "${warned.rule}" matches no known tool — check for typos.`,
  'never matches': (warned) =>
    fileToolRuleWarning({
      behavior: warned.behavior,
      source: sourceOf(warned),
      rule: warned.rule,
      use: warned.use ?? 'Edit',
      replacement: warned.replacement ?? '',
    }),
};

const issueOf = (warned: RuleWarning): PluginIssue =>
  warning(`permissions.${warned.behavior}`, STDERR[warned.kind](warned));

/**
 * The startup warnings of a rule list (errors.md, "Configuration warnings"), in the words of a
 * non-interactive text run; `ruleWarnings` gives the same warnings as values.
 */
export function ruleIssues(
  rules: readonly PermissionRule[],
  knownTools: ReadonlySet<string>,
): readonly PluginIssue[] {
  return rules.flatMap((rule) => [
    ...ruleWarnings([rule], knownTools).map(issueOf),
    ...wildcardBeforeSubcommand(rule),
  ]);
}
