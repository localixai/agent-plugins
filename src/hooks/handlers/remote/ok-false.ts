import type { HookEvent } from '../../../protocol/official.ts';
import { coreModule } from '../../../wasm/bridge.ts';
import type { ModelRun } from './types.ts';

/**
 * What `ok: false` of a `prompt` or `agent` hook does on each event (hooks.md, "Response schema":
 * "What happens on `ok: false` depends on the event"). The run says what the model decided; this
 * says what the event does with it, as data the interpreter turns into its outcome.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/ok_false.rs`,
 * docs/RUST.md).
 */

const call = coreModule('hooks/handlers/remote/ok-false');

/** The events that run `prompt` and `agent` hooks (`PermissionRequest`: prompt only). */
export type PromptEvent = Extract<
  HookEvent,
  | 'PermissionDenied'
  | 'PermissionRequest'
  | 'PostToolBatch'
  | 'PostToolUse'
  | 'PostToolUseFailure'
  | 'PreToolUse'
  | 'Stop'
  | 'SubagentStop'
  | 'TaskCompleted'
  | 'TaskCreated'
  | 'TeammateIdle'
  | 'UserPromptExpansion'
  | 'UserPromptSubmit'
>;

/** Where the reason goes. */
export type Delivery =
  /** "Fed back to Claude as its next instruction". */
  | 'claude-instruction'
  /** "Returned to Claude as a tool error". */
  | 'claude-tool-error'
  /** "The reason appears in the chat as a warning line". */
  | 'user-warning'
  /** "Fed back to the teammate". */
  | 'teammate';

export type OkFalseEffect =
  /** No effect: `ok: true`, or an event on which `ok: false` changes nothing. */
  | { readonly kind: 'none' }
  /** `Stop` and `SubagentStop` with `impossible: true`: "Claude Code allows the stop and the turn ends". */
  | { readonly kind: 'allow-stop' }
  | {
      readonly kind: 'block';
      readonly reason: string;
      /** The turn ends (or the teammate stops) instead of going on. */
      readonly endsTurn: boolean;
      readonly delivery: Delivery;
      /** `PreToolUse`: "the tool call is denied". */
      readonly deniesTool: boolean;
    };

/** What a rule reads: the verdict, and how the hook was configured. */
export interface OkFalseCase {
  readonly reason: string;
  /** `continueOnBlock: true` of a prompt hook, or an agent hook (it behaves as if set). */
  readonly continues: boolean;
  /** The model said `impossible` and the hook type reads it (prompt hooks only). */
  readonly impossible: boolean;
  /** `TaskCompleted` fired because a teammate stopped, not because a task was marked completed. */
  readonly teammateStopped: boolean;
}

type Rule = (found: OkFalseCase) => OkFalseEffect;

/** `OK_FALSE_RULES`' events, in the table's order: each rule is the core's. */
const PROMPT_EVENTS: readonly PromptEvent[] = call('OK_FALSE_RULES');

// oxlint-disable-next-line typescript/no-unsafe-type-assertion -- `Object.fromEntries` types its keys as `string`; the core's list is exactly the 13 events.
export const OK_FALSE_RULES: { readonly [E in PromptEvent]: Rule } = Object.fromEntries(
  PROMPT_EVENTS.map((event): [PromptEvent, Rule] => [
    event,
    (found) => call('okFalseRule', event, found),
  ]),
) as { readonly [E in PromptEvent]: Rule };

export const isPromptEvent = (event: HookEvent): event is PromptEvent =>
  call('isPromptEvent', event);

export interface OkFalseOptions {
  /** The hook is an `agent` hook: "the way it handles a prompt hook with `continueOnBlock: true`"; no `impossible`. */
  readonly agent: boolean;
  /** `continueOnBlock` of the prompt hook. */
  readonly continueOnBlock?: boolean;
  /** `TaskCompleted` only: a teammate stopping fired it. Without it the event is a `TaskUpdate`. */
  readonly teammateStopped?: boolean;
}

/** What the verdict of a prompt or agent hook does on `event`. */
export const okFalseEffect = (
  event: PromptEvent,
  run: ModelRun,
  options: OkFalseOptions,
): OkFalseEffect => call('okFalseEffect', event, run, options);
