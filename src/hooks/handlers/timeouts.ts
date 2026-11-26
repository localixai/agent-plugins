import type { Environment } from '../../ports/index.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { RegisteredHandler } from '../contracts.ts';

/**
 * How long a handler may run (hooks.md, "Common fields", `timeout`): the kit computes the limit
 * and hands it to the port, which enforces it and reports `timedOut`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/timeouts.rs`,
 * docs/RUST.md).
 */

const call = coreModule('hooks/handlers/timeouts');

/** The limits `SessionEnd` hooks share: one budget for the event, and the default of each hook. */
export interface SessionEndLimits {
  readonly budgetMs: number;
  readonly defaultMs: number;
}

/** What an event adds to the timeouts of its handlers; computed once per dispatch. */
export interface TimeoutPlan {
  readonly sessionEnd?: SessionEndLimits;
  /** An interrupt caps every hook that has a limit (hooks.md, "PostToolUse after an interrupt"). */
  readonly interruptCapMs?: number;
}

/** The plan of one dispatch: `selected` are the handlers of `event` that will run. */
export const planTimeouts = (
  event: HookEvent,
  selected: readonly RegisteredHandler[],
  environment: Environment,
  interruptCapMs?: number,
): TimeoutPlan => call('planTimeouts', event, selected, environment, interruptCapMs);

/**
 * The limit in milliseconds the port enforces on this handler, or `undefined` for none (an `async`
 * command hook). Judgment: a `timeout` that is not a positive number counts as unset.
 */
export const handlerTimeoutMs = (entry: RegisteredHandler, plan: TimeoutPlan): number | undefined =>
  call('handlerTimeoutMs', entry, plan);
