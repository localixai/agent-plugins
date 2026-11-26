/**
 * The tool matchers that select exactly one core tool profile (Agent Plugins Extended §18.3.4,
 * §18.4.3 item 4, the table of §18.2.7): a Claude Code matcher converts when it names one profile's
 * tools, and a core handler that a converted adapter limits to one profile converts back to that
 * matcher.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/profiles.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('convert/profiles');

export const PROFILE_NAMES: readonly [
  'shell',
  'file.read',
  'file.modify',
  'web.fetch',
  'web.search',
  'mcp',
] = call('PROFILE_NAMES');

export type ProfileName = (typeof PROFILE_NAMES)[number];

export const isProfileName = (value: string | undefined): value is ProfileName =>
  call('isProfileName', value);

/** The matcher that selects a profile's tools. */
export const matcherOf = (profile: ProfileName): string => call('matcherOf', profile);

/** A matcher that matches every tool (hooks.md, "Matcher patterns": omitted, `""` or `*`). */
export const matchesEverything = (matcher: string | undefined): boolean =>
  call('matchesEverything', matcher);

/** The one profile a matcher selects, in any order of its names; `undefined` for any other matcher. */
export const profileOfMatcher = (matcher: string): ProfileName | undefined =>
  call('profileOfMatcher', matcher);
