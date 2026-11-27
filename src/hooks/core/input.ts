import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { CoreCallFacts, CoreOccurrence } from './types.ts';

/**
 * Which Claude Code occurrences are also core occurrences, and the core input each gives (Agent
 * Plugins Extended §18.1.2, the table and "Inputs"; §18.2): exactly the core fields (core
 * §7.3.3–§7.3.8), the common ones with the values Claude Code's handlers get, never a Claude Code
 * field. An event without a core counterpart, and an occurrence the table excludes, gives none.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/core/input.rs`, docs/RUST.md).
 */

export interface OccurrenceContext {
  readonly sessionId: string;
  readonly cwd: string;
  /** The hooks fire in a subagent: core `SessionStart` does not run for one (core §7.3.4). */
  readonly subagent: boolean;
  readonly facts: CoreCallFacts;
}

type Facts = object;

const call = coreModule('hooks/core/input');

/** Whether a `UserPromptSubmit` turn is one only the host can classify and the host did not. */
export const needsTurnOrigin = (event: HookEvent, facts: Facts, core: CoreCallFacts): boolean =>
  call('needsTurnOrigin', event, facts, core);

/** The core occurrence of a Claude Code dispatch, or `undefined` when the core event does not fire. */
export const coreOccurrence = (
  event: HookEvent,
  facts: Facts,
  context: OccurrenceContext,
): CoreOccurrence | undefined => call('coreOccurrence', event, facts, context);
