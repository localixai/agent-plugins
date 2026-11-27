/**
 * The facts of every event, read from its input by the event's schema, one per member of the
 * official `HookEvent` union.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/input/facts.rs`, docs/RUST.md).
 * The schema of each event's facts stays in the core (`facts_*.rs`): a schema cannot cross.
 */
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { EventFactsOf } from '../contracts.ts';

const call = coreModule('hooks/input/facts');

/** `event`'s facts read from `input` (zod's `parse`): throws the issues when the schema refuses it. */
export const parseFacts = <E extends HookEvent>(event: E, input: unknown): EventFactsOf<E> =>
  call('parseFacts', event, input);
