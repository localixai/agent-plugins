import { coreModule } from '../wasm/bridge.ts';
import type { RuleFormatId } from './specifier-formats.ts';
import type { MatchContext, PermissionRule, PreparedCall } from './types.ts';

/**
 * The matcher of each rule format.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/matchers.rs`, docs/RUST.md).
 */
const call = coreModule('rules/matchers');

/**
 * Matches the text or domain specifier of one rule against one call. A rule reaches its matcher
 * only after its tool name matched and it is neither bare nor a parameter rule, so the matcher
 * reads `rule.specifier` and the call's input, and decides by `rule.behavior` where the docs
 * differ (an allow rule needs every Bash subcommand covered, a deny or ask rule any).
 */
export type SpecifierMatcher = (
  rule: PermissionRule,
  call: PreparedCall,
  ctx: MatchContext,
) => boolean;

export type SpecifierMatchers = { readonly [F in RuleFormatId]: SpecifierMatcher };

/**
 * One matcher per format. What an unwritten matcher answers: it fails on the safe side. An allow
 * rule that matches nothing approves nothing; a deny or ask rule covers every call of its tool until
 * the real matcher exists.
 */
export const SPECIFIER_MATCHERS: SpecifierMatchers = call('SPECIFIER_MATCHERS');
