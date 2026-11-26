/**
 * MCP rules (permissions.md, "MCP"). A tool's callable name is `mcp__<server>__<tool>`, where the
 * server is the name Claude Code gives it (mcp.md: every character outside `A-Z`, `a-z`, `0-9`,
 * `_` and `-` becomes `_`). A rule names the server (`mcp__puppeteer`, `mcp__puppeteer__*`) or one
 * tool (`mcp__puppeteer__puppeteer_navigate`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/target/mcp.rs`, docs/RUST.md).
 */
import type { PermissionBehavior } from '../../protocol/official.ts';
import { mcpToolName, normalizeMcpServerName, pluginMcpServerName } from '../../format/mcp-tool.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PermissionRule } from '../types.ts';

const call = coreModule('rules/target/mcp');

/**
 * Whether a rule's tool name, written without a `*`, covers the call of `toolName`. The name itself
 * always does. An `mcp__` name also covers the tools of the server it names; the three behaviours
 * differ for a server whose name holds `__` (permissions.md: as a deny or ask rule `mcp__reports`
 * and `mcp__reports__v2` also cover every tool of `reports__v2`, as an allow rule they do not).
 */
export const mcpToolRuleMatches = (
  ruleToolName: string,
  toolName: string,
  behavior: PermissionBehavior,
): boolean => call('mcpToolRuleMatches', ruleToolName, toolName, behavior);

/** Whether `rule` is a deny rule that names a whole built-in tool and `toolName` is that tool's Cowork twin. */
export const denyCoversCoworkTool = (rule: PermissionRule, toolName: string): boolean =>
  call('denyCoversCoworkTool', rule, toolName);

export { mcpToolName, normalizeMcpServerName, pluginMcpServerName };
