import type { DocumentedTool } from '../../protocol/tool-names.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { RuleFormatId } from './rule-formats.ts';

const call = coreModule('rules/tools/catalog');

/**
 * What tools-reference.md says about a tool in rules: the "Permission required" column of the tools
 * table and the "Rule format" table ("Configure tools with permission rules and hooks"), whose
 * "Applies to" column names the tools each format covers; "Tools not listed here ... accept only
 * the bare tool name with no specifier".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/catalog.rs`, docs/RUST.md).
 */
export interface CatalogToolEntry {
  /** The tool prompts in Manual mode for paths inside the working directory. */
  readonly permissionRequired: boolean;
  readonly ruleFormat: RuleFormatId;
}

/** One entry per documented tool: a tool the table adds fails the typecheck until it is described here. */
export const TOOL_CATALOG: { readonly [T in DocumentedTool]: CatalogToolEntry } =
  call('TOOL_CATALOG');

/** The entry of a documented tool, or `undefined` for any other name (an MCP tool, a typo). */
export const catalogEntry = (toolName: string): CatalogToolEntry | undefined =>
  call('catalogEntry', toolName);

/**
 * The tool names a rule can name without a typo warning: the documented tools Claude Code still
 * has and the SDK's own tools the docs leave undescribed. Pass it to `ruleIssues`.
 */
export const KNOWN_TOOLS: ReadonlySet<string> = call('KNOWN_TOOLS');

/** Whether `toolName` is a built-in tool Claude Code has (the removed `TaskOutput` is not). */
export const isKnownTool = (toolName: string): boolean => call('isKnownTool', toolName);

/**
 * `DecideContext.requiresPermission` for the kit's catalog: the "Permission required" column for a
 * documented tool, and `true` for any other name (an MCP tool prompts "on first use of each tool",
 * mcp.md; spec/judgments.md).
 */
export const toolRequiresPermission = (toolName: string): boolean =>
  call('toolRequiresPermission', toolName);
