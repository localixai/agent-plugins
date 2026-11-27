import type { PluginIssue } from '../../format/issues.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HookOutcome } from '../outcome.ts';
import type { CoreOccurrence, CoreRun } from './types.ts';

/**
 * One occurrence's outcome with its core side added (Agent Plugins Extended §18.1.2, "Combining
 * results"): Claude Code's outcome is what the engine made of its handlers, unchanged in what it
 * says of them (principle 1); the core results join it by the event's rule. A failed core handler
 * contributes nothing but its notice (J18.4). Every Claude Code handler precedes every core one in
 * handler order, and core handlers follow the order their packages loaded (J18.3).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/core/combine.rs`, docs/RUST.md).
 */

export interface CoreSide {
  readonly occurrence: CoreOccurrence;
  readonly runs: readonly CoreRun[];
  /** Every successful `additional_context`, already capped and with its reminder tags escaped. */
  readonly contexts: readonly string[];
  readonly issues: readonly PluginIssue[];
}

const call = coreModule('hooks/core/combine');

/** The outcome of the occurrence with the core side combined into it. */
export const combineCore = <E extends HookEvent>(
  outcome: HookOutcome<E>,
  side: CoreSide,
  facts: object,
): HookOutcome<E> => call('combineCore', outcome, side, facts);
