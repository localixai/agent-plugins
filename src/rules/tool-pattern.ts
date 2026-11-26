/**
 * A rule's tool-name glob and the tool-name position of a rule.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tool_pattern.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import { memoize } from './memo.ts';
import type { PermissionRule } from './types.ts';

const call = coreModule('rules/tool-pattern');

/**
 * A `*` pattern as a regular expression that must match the whole text: `*` stands for any
 * sequence of characters, spaces and slashes included, and every other character is literal.
 */
export const wildcardRegExp = (pattern: string, flags = ''): RegExp =>
  compiled(`${flags}/${pattern}`);

/** One RegExp per flags and pattern, as the callers in TypeScript test one many times. */
const compiled = memoize((key): RegExp => {
  const cut = key.indexOf('/');
  return call('wildcardRegExp', key.slice(cut + 1), key.slice(0, cut));
});

/**
 * Whether an allow rule's tool-name glob names one server: permissions.md ("Tool name wildcards")
 * accepts one "only after a literal `mcp__<server>__` prefix", so `mcp__puppeteer__*` and
 * `mcp__github__get_*` do and `*`, `B*` and `mcp__*` do not. The server is the text between
 * `mcp__` and the next `__`, and it must hold no `*`.
 */
export const isAnchoredAllowGlob = (toolName: string): boolean =>
  call('isAnchoredAllowGlob', toolName);

/**
 * Whether the tool-name position of `rule` covers `toolName`: a glob matching it whole, else the
 * name itself, an `mcp__` server name covering its tools (`mcpToolRuleMatches`), or a deny rule that
 * names `Bash` or `WebFetch` whole covering the Cowork twin of that tool.
 */
export const toolNameMatches = (rule: PermissionRule, toolName: string): boolean =>
  call('toolNameMatches', rule, toolName);
