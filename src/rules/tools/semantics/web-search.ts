/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/web_search.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/web-search');

/**
 * "Claude can scope results with `allowed_domains` to include only certain hosts, or
 * `blocked_domains` to exclude them. The two lists can't be combined in a single call."
 * (tools-reference.md, "WebSearch tool behavior"). A call with both lists filled is one error at
 * `tool_input`. An empty list is no list, a value that is not an array is left to `parseToolInput`
 * and the wording is a judgment (spec/judgments.md). How an entry matches a result's host is not in
 * the docs and is not modelled.
 */
export const validateWebSearchDomains = (input: unknown): readonly PluginIssue[] =>
  call('validateWebSearchDomains', input);
