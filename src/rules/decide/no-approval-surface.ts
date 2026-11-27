/**
 * The messages of `permissions.denyInsteadOfPrompting` (settings-reference.md, "Messages from
 * `denyInsteadOfPrompting`"), verbatim: they report refused calls and nothing waits on them. The
 * host writes them where the docs put them; the kit holds the texts and the count of refusals.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/no_approval_surface.rs`,
 * docs/RUST.md).
 */
import type { SDKPermissionDeniedMessage, SDKResultError } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/decide/no-approval-surface');

/** "The denial Claude reads can open with this text." */
export const NO_APPROVAL_SURFACE_DENIAL: string = call('NO_APPROVAL_SURFACE_DENIAL');

/** The `decision_reason` of a `permission_denied` message in `stream-json` output. */
export const NO_APPROVAL_SURFACE_DECISION_REASON: NonNullable<
  SDKPermissionDeniedMessage['decision_reason']
> = call('NO_APPROVAL_SURFACE_DECISION_REASON');

/** "The turn ends after 20 calls in a row that the classifier blocked or a safety check refused". */
export const REFUSAL_STOP_LIMIT: number = call('REFUSAL_STOP_LIMIT');

/** What the person sees when those refusals end the turn. */
export const REFUSAL_STOP_MESSAGE: string = call('REFUSAL_STOP_MESSAGE');

/** "A `-p` run reports that turn as `error_during_execution`." */
export const REFUSAL_STOP_RESULT: SDKResultError['subtype'] = call('REFUSAL_STOP_RESULT');

/** One call of an auto mode turn: refused by the classifier or a safety check, or work that went ahead. */
export type RefusalEvent = 'refused' | 'went-ahead';

/** Where the turn stands after a call: it goes on, or it ends with the message. */
export type RefusalVerdict =
  | { readonly outcome: 'continue' }
  | { readonly outcome: 'stop'; readonly message: string };

export interface RefusalStreak {
  /** Records a call and says whether the turn ends. */
  record(event: RefusalEvent): RefusalVerdict;
}

/**
 * The count of one turn "in auto mode with the setting on": 20 refusals in a row, with "no work
 * went ahead in between", end it. A host makes one per turn, and only in that case.
 */
export const createRefusalStreak = (): RefusalStreak => call('createRefusalStreak');
