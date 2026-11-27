/**
 * The 10,000-character cap on each string of each hook (hooks.md, "JSON output"): the JSON fields
 * `additionalContext`, `systemMessage` and `initialUserMessage`, each on its own, and the plain
 * stdout whole. Every hook is measured alone, "even when several hooks run for the same event".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/limit.rs`, docs/RUST.md).
 */
import type { ScratchFiles } from '../../ports/scratch-files.ts';
import type { HookEvent } from '../../protocol/official.ts';
import type { BackgroundDelivery } from '../verdict/types.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/limit');

export interface LimitRequest {
  readonly scratch?: ScratchFiles;
  readonly signal: AbortSignal;
}

/**
 * What a background hook delivers, under the same cap and escape as a foreground hook's text: "A
 * hook's `additionalContext`, `systemMessage` ... strings ... are capped at 10,000 characters"
 * whether it ran in the foreground or not, and the context reaches Claude with its reminder tags
 * escaped (`escapeReminderTags`). A delivery carries no issues: a text cut for want of a
 * `ScratchFiles` port says so in its own marker.
 */
export const limitDelivery = (
  event: HookEvent,
  delivery: BackgroundDelivery,
  request: LimitRequest,
): Promise<BackgroundDelivery> => call.async('limitDelivery', event, delivery, request);
