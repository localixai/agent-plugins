import type { PluginIssue } from '../../format/issues.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { EventFactsOf } from '../contracts.ts';

/**
 * Fields an event's input has only in some cases (hooks.md, the input tables). A host that supplies
 * one where the docs say it is absent gets it dropped, with a warning, so a hook never sees a field
 * Claude Code would not send.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/input/conditional_fields.rs`,
 * docs/RUST.md).
 */

const call = coreModule('hooks/input/conditional-fields');

interface Refined<E extends HookEvent> {
  readonly facts: EventFactsOf<E>;
  readonly issues: readonly PluginIssue[];
}

/** `facts` as `event`'s input has them: the conditional fields kept or dropped as the docs say. */
export const applyConditionalFields = <E extends HookEvent>(
  event: E,
  facts: EventFactsOf<E>,
): Refined<E> => call('applyConditionalFields', event, facts);
