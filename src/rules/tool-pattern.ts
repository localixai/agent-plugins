/**
 * A rule's tool-name glob and the tool-name position of a rule.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tool_pattern.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PermissionRule } from './types.ts';

const call = coreModule('rules/tool-pattern');

/**
 * Whether the tool-name position of `rule` covers `toolName`: a glob matching it whole, else the
 * name itself, an `mcp__` server name covering its tools (`mcpToolRuleMatches`), or a deny rule that
 * names `Bash` or `WebFetch` whole covering the Cowork twin of that tool.
 */
export const toolNameMatches = (rule: PermissionRule, toolName: string): boolean =>
  call('toolNameMatches', rule, toolName);
