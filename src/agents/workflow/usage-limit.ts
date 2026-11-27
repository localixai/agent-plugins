/**
 * When a run hits your usage limit (workflows.md, "When a run hits your usage limit"): the run pauses
 * instead of failing the agent, when all of the conditions hold; when one does not, the affected agent
 * fails. Pure over facts the host holds.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/workflow/usage_limit.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('agents/workflow/usage-limit');

export const SESSION_KINDS: readonly [
  'interactive',
  'non-interactive',
  'background',
  'remote-control',
  'agent-team-teammate',
] = call('SESSION_KINDS');
export type SessionKind = (typeof SESSION_KINDS)[number];

export const NO_WAIT_CAUSES: readonly [
  'session-not-interactive',
  'no-subscription',
  'auto-continue-off',
  'reset-too-far',
  'waited-twice',
  'old-release',
] = call('NO_WAIT_CAUSES');
export type NoWaitCause = (typeof NO_WAIT_CAUSES)[number];

export interface UsageLimitFacts {
  readonly session: SessionKind;
  /** Signed in with a claude.ai subscription. */
  readonly subscription: boolean;
  /** `autoContinueAtUsageLimit` is on. */
  readonly autoContinue: boolean;
  /** Milliseconds until the limit resets. A weekly limit can reset further out. */
  readonly msUntilReset: number;
  /** How many times the run has already waited. */
  readonly waitsSoFar: number;
  /** The release has the wait (Claude Code v2.1.271 or later); "on earlier versions, the affected agents fail". */
  readonly releaseHasWait: boolean;
}

export type UsageLimitDecision =
  | {
      /** The agents that hit the limit wait for the reset, and "no new agents start" while it lasts. */
      readonly pauses: true;
      /** Shown on the progress line and in the `/workflows` header. */
      readonly msUntilReset: number;
    }
  | { readonly pauses: false; readonly failsAgent: true; readonly causes: readonly NoWaitCause[] };

/**
 * Whether the run waits for the reset or the agent that hit the limit fails. Asked again during a
 * wait with the current facts: "If you turn [`autoContinueAtUsageLimit`] off during a wait, the wait
 * ends and the waiting agents fail."
 */
export const usageLimitDecision = (facts: UsageLimitFacts): UsageLimitDecision =>
  call('usageLimitDecision', facts);
