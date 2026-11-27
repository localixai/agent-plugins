/**
 * How a tool reads what follows its name inside the parentheses of a rule: the "Rule format" table
 * of tools-reference.md ("Configure tools with permission rules and hooks"). One id per format;
 * every part that differs per format (matching, syntax details) is a registry typed over
 * `RuleFormatId`, so a format added here fails the typecheck until each registry handles it. Which
 * tool uses which format is the catalog's `ruleFormat` (`catalog.ts`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/rule_formats.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/tools/rule-formats');

export const RULE_FORMATS: readonly [
  'command',
  'powershell-command',
  'read-path',
  'edit-path',
  'skill-name',
  'subagent-type',
  'domain',
  'bare',
] = call('RULE_FORMATS');

export type RuleFormatId = (typeof RULE_FORMATS)[number];
