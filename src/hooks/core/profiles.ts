/**
 * The portable view of a pending Claude Code call (Agent Plugins Extended §18.2.7; core §7.3.6): a
 * table by Claude Code tool name of 2.1.321, each entry deciding from the call's defined operation
 * and input whether its profile applies. A tool the table does not name gets none (`Glob`, `Grep`,
 * `Agent`, `Skill`, `AskUserQuestion` and every bookkeeping tool), and so does a call the entry's
 * criteria exclude (a `Monitor` with `ws`, a `WebFetch` of a relative URL).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/core/profiles.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { CoreCallFacts, CoreToolProfile } from './types.ts';

export interface ProfileContext {
  /** The session's working directory: a relative `notebook_path` is made absolute against it. */
  readonly cwd: string;
  readonly facts: CoreCallFacts;
}

const call = coreModule('hooks/core/profiles');

/** The profile of a pending call, or `undefined` when none applies. */
export const pendingProfile = (
  toolName: string,
  toolInput: unknown,
  context: ProfileContext,
): CoreToolProfile | undefined => call('pendingProfile', toolName, toolInput, context);
