import type { CoreHookEvent } from '../../format/core/hooks.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ProfileName } from '../profiles.ts';

/**
 * Which Claude Code matchers convert, per core event (Agent Plugins Extended §18.4.3 item 4): on the
 * tool events a matcher that selects one profile's tools, or everything; on `SessionStart` the new
 * sessions the core fires on (`startup|clear|fork`, J18.6) or everything, which then runs on new
 * sessions only; `UserPromptSubmit` and `Stop` ignore a matcher in Claude Code (hooks.md, "Matcher
 * patterns"), so any converts.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_core/hook_matchers.rs`,
 * docs/RUST.md).
 */
const call = coreModule('convert/to-core/hook-matchers');

export type MatcherReading =
  | { readonly profile?: ProfileName; readonly narrowed?: string }
  | { readonly problem: string };

export const readMatcher = (event: CoreHookEvent, matcher: string | undefined): MatcherReading =>
  call('readMatcher', event, matcher);
