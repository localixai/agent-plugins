import type { PluginIssue } from '../../format/issues.ts';
import type { HookInputOf } from '../../protocol/hook-events.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { EventFactsOf, HookSession } from '../contracts.ts';
import type { CommonInput } from './common.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/input/build.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/input/build');

/** The input of an event: the official shape, plus the documented `scratchpad_dir`. */
export type BuiltHookInput<E extends HookEvent> = HookInputOf<E> &
  Pick<CommonInput, 'scratchpad_dir'>;

export interface BuiltInput<E extends HookEvent> {
  /** `undefined` when the facts do not make a valid input: the hook does not run. */
  readonly input: BuiltHookInput<E> | undefined;
  /** Errors for facts that fail the official shape, warnings for what was dropped. */
  readonly issues: readonly PluginIssue[];
}

/**
 * The JSON a command hook reads on stdin and an HTTP hook receives as its POST body, from the
 * event's facts and the session (hooks.md, "Hook input and output"): the common input fields, then
 * the event's own. The facts are checked against the official input type of the event; a wrong
 * type or enum value is an error issue and no input, a field the event does not have or has only in
 * other cases (`conditional-fields.ts`) is dropped with a warning.
 */
export const buildHookInput = <E extends HookEvent>(
  event: E,
  facts: EventFactsOf<E>,
  session: HookSession,
): BuiltInput<E> => call('buildHookInput', event, facts, session);

/** One JSON text for both transports: stdin of a command hook, body of an HTTP hook. */
export const serializeHookInput = (input: HookInputOf<HookEvent>): string =>
  call('serializeHookInput', input);
