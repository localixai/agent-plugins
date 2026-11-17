import type { CoreHookEvent } from '../../format/core/hooks.ts';
import { matchesEverything, profileOfMatcher, type ProfileName } from '../profiles.ts';

/**
 * Which Claude Code matchers convert, per core event (Agent Plugins Extended §18.4.3 item 4): on the
 * tool events a matcher that selects one profile's tools, or everything; on `SessionStart` the new
 * sessions the core fires on (`startup|clear|fork`, J18.6) or everything, which then runs on new
 * sessions only; `UserPromptSubmit` and `Stop` ignore a matcher in Claude Code (hooks.md, "Matcher
 * patterns"), so any converts.
 */

export type MatcherReading =
  | { readonly profile?: ProfileName; readonly narrowed?: string }
  | { readonly problem: string };

const NEW_SESSIONS = ['startup', 'clear', 'fork'];

const toolMatcher = (matcher: string | undefined): MatcherReading => {
  if (matchesEverything(matcher)) return {};
  const profile = profileOfMatcher(matcher ?? '');
  return profile === undefined
    ? { problem: `the matcher "${matcher}" selects no single tool profile` }
    : { profile };
};

function sessionMatcher(matcher: string | undefined): MatcherReading {
  if (matchesEverything(matcher)) {
    return {
      narrowed: 'it runs on new sessions only (startup, clear, fork), not on resume or compact',
    };
  }
  const sources = (matcher ?? '').split('|');
  const same =
    sources.length === NEW_SESSIONS.length &&
    NEW_SESSIONS.every((source) => sources.includes(source));
  return same
    ? {}
    : {
        problem: `the matcher "${matcher}" selects other sessions than the new ones the core fires on`,
      };
}

const ANY: () => MatcherReading = () => ({});

const MATCHERS: { readonly [E in CoreHookEvent]: (matcher: string | undefined) => MatcherReading } =
  {
    SessionStart: sessionMatcher,
    UserPromptSubmit: ANY,
    PreToolUse: toolMatcher,
    PostToolUse: toolMatcher,
    Stop: ANY,
  };

export const readMatcher = (event: CoreHookEvent, matcher: string | undefined): MatcherReading =>
  MATCHERS[event](matcher);
