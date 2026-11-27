import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PermissionRule } from './types.ts';

/**
 * The startup warnings of a rule list (errors.md, "Configuration warnings"), in the words of a
 * non-interactive text run: the "matches no known tool" and "is not matched by file permission
 * checks" warnings of `ruleWarnings`, and the wildcard-before-subcommand warning.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/issues.rs`, docs/RUST.md).
 */
const call = coreModule('rules/issues');

/**
 * The startup warnings of a rule list (errors.md, "Configuration warnings"), in the words of a
 * non-interactive text run; `ruleWarnings` gives the same warnings as values.
 */
export const ruleIssues = (
  rules: readonly PermissionRule[],
  knownTools: ReadonlySet<string>,
): readonly PluginIssue[] => call('ruleIssues', rules, knownTools);
