import type { HookEvent } from '../../protocol/official.ts';
import type { MergedOutput, TaggedOutput } from '../verdict/types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The merge of every hook's output on one event (hooks.md, "JSON output", "Decision control"):
 * what each event honours (`OUTPUT_CAPS`), the universal fields, the context every hook adds, and
 * the event's own rule from `MERGERS`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/merge.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/output/merge');

/** `facts` carry what a rule needs from the event (`source`, `tool_name`). */
export const mergeOutputs = (
  event: HookEvent,
  outputs: readonly TaggedOutput[],
  facts: object = {},
): MergedOutput => call('mergeOutputs', event, outputs, facts);
