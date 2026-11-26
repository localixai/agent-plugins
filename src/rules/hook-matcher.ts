/**
 * How a hook's `matcher` is evaluated (hooks.md, "Matcher patterns"). The characters a matcher
 * holds decide how it reads:
 *
 * - `"*"`, `""` or omitted: matches every occurrence of the event;
 * - only exact-match characters: an exact string, or a list of exact strings;
 * - any other character: a JavaScript regular expression, tested unanchored
 *   (`RegExp.prototype.test`, so it succeeds on a match anywhere in the value).
 *
 * `FileChanged` and `StopFailure` use the narrower `narrow` set, so a hyphen, space or comma sends
 * their matcher to the regular-expression path and only `|` separates alternatives. Every other
 * event with matcher support uses `standard`. Which value an event is matched against, and that an
 * event without matcher support ignores the matcher, is the caller's (hooks/select).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/hook_matcher.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

export type MatcherMode = 'standard' | 'narrow';

const call = coreModule('rules/hook-matcher');

/**
 * Whether `matcher` selects `value`. A match-all matcher selects even an event that has no value
 * to filter on; any other matcher needs a value (judgment: it matches nothing without one).
 */
export const matchesMatcher = (
  matcher: string | undefined,
  value: string | undefined,
  mode: MatcherMode = 'standard',
): boolean => call('matchesMatcher', matcher, value, mode);
