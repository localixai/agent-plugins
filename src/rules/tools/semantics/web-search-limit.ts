/**
 * WebSearch's session limit and how many calls it still allows (tools-reference.md, "Session search
 * limit").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/web_search_limit.rs`,
 * docs/RUST.md).
 */
import type { Environment } from '../../../ports/environment.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/web-search-limit');

export interface WebSearchSession {
  /** An interactive terminal session; `false` for `-p` and the Agent SDK. */
  readonly interactive: boolean;
}

export interface WebSearchLimits {
  /** The most calls the count may reach. */
  readonly cap: number;
  /** Calls given back per hour. */
  readonly refillPerHour: number;
  /** "In non-interactive mode with `-p` and in the Agent SDK, the limit applies per turn, not per session." */
  readonly perTurn: boolean;
}

/**
 * The limits of WebSearch (tools-reference.md, "Session search limit"; env-vars.md).
 * `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` "accepts a positive whole number"; anything else is
 * ignored, so the cap "can be raised but not turned off". `CLAUDE_CODE_WEB_SEARCH_REFILLS_PER_HOUR`
 * takes plain digits (`0` included); the default is 100 in an interactive session and 0, no
 * refill, in a non-interactive one. A number below 200 lowers the cap (spec/judgments.md).
 */
export const webSearchLimits = (
  env: Pick<Environment, 'get'>,
  session: WebSearchSession,
): WebSearchLimits => call('webSearchLimits', env, session);

export interface WebSearchUsage {
  /** Calls counted so far in the session, or in the turn when the limits are per turn. */
  readonly used: number;
  /** Milliseconds since the count started. */
  readonly elapsedMs: number;
}

/**
 * How many more WebSearch calls are allowed. The count refills at the hourly rate, continuously and
 * rounded down, and never gives back more than was used (spec/judgments.md: the docs say "about 100
 * calls per hour" and no more). At zero a call "did nothing" and Claude is told to carry on.
 */
export const webSearchAllowance = (limits: WebSearchLimits, usage: WebSearchUsage): number =>
  call('webSearchAllowance', limits, usage);
