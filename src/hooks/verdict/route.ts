import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { BlockInfo, BlockVia, HandlerVerdict } from './types.ts';

/**
 * Where the text of a block goes (hooks.md, "Exit code 2 behavior per event" and the event
 * sections): the route of an event's exit-2 row, applied to every block on it, since a failure
 * "does what exit code 2 does on that event" and a blocking decision "routes the same way".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/route.rs`, docs/RUST.md).
 */
const call = coreModule('hooks/verdict/route');

/** The text a block carries, before the route decides whether anyone is shown it. */
export interface BlockText {
  readonly reason: string;
  readonly message: string;
}

/** A block on `event`: the text stays with the hook when the docs show it to no one. */
export const routedBlock = (
  event: HookEvent,
  via: BlockVia,
  text: BlockText,
  condition?: BlockInfo['condition'],
): BlockInfo => call('routedBlock', event, via, text, condition);

/**
 * "You see it as a warning in the transcript, and it stays in the conversation, so Claude sees it
 * when the conversation continues": a block routed to both also leaves a notice and feedback.
 */
export const shownWith = (
  handlerId: string,
  block: BlockInfo,
): Pick<HandlerVerdict, 'notice' | 'feedback'> => call('shownWith', handlerId, block);
