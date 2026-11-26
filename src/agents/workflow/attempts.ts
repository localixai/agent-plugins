import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * When an agent stalls and restarts (workflows.md, "When an agent stalls and restarts"): an agent
 * whose output stops arriving starts over from the same prompt, at most five times, so at most six
 * attempts. Pure decisions over the endings of the attempts so far; the host measures time and runs
 * the agent.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/workflow/attempts.rs`, docs/RUST.md).
 */
const call = coreModule('agents/workflow/attempts');

/** How an attempt ended without finishing: the whole stall window without output, a response stream that went silent, or a restart asked for. */
export const ATTEMPT_ENDINGS: readonly ['stalled', 'lost-reply', 'restarted'] =
  call('ATTEMPT_ENDINGS');
export type AttemptEnding = (typeof ATTEMPT_ENDINGS)[number];

/** The `/workflows` labels of an attempt: `(retry 1)` after the name, `attempt 2 (stalled)` in the detail. */
export interface AttemptLabels {
  readonly nameSuffix: string;
  readonly detail: string;
}

export interface Restart {
  readonly restart: true;
  /** The number of the attempt that starts, from 2. */
  readonly attempt: number;
  readonly labels: AttemptLabels;
}

/** Why the `agent()` call failed, and the error whose start says why. */
export const FAILURE_CAUSES: readonly ['stalled', 'lost-reply', 'abandoned'] =
  call('FAILURE_CAUSES');
export type WorkflowFailureCause = (typeof FAILURE_CAUSES)[number];

export interface AgentFailure {
  readonly restart: false;
  readonly cause: WorkflowFailureCause;
  readonly issue: PluginIssue;
}

export type AttemptVerdict = Restart | AgentFailure;

/** The labels of attempt `attempt` (from 1), which follows an attempt that ended as `previous`. */
export const attemptLabels = (
  attempt: number,
  previous: AttemptEnding | undefined,
): AttemptLabels => call('attemptLabels', attempt, previous);

/**
 * What follows an attempt that ended without finishing. `endings` holds how every attempt so far
 * ended, the one just ended last. Up to the fifth attempt the agent starts over; when the sixth ends the
 * same way the `agent()` call fails.
 */
export const afterAttempt = (endings: readonly AttemptEnding[]): AttemptVerdict =>
  call('afterAttempt', endings);

/** Whether a restart asked for on attempt `attempt` is allowed: it counts toward the five. */
export const canRestartOnRequest = (attempt: number): boolean =>
  call('canRestartOnRequest', attempt);

export interface StallWindowInput {
  /** `stallMs` passed on this agent's `agent()` call. */
  readonly stallMs?: number | undefined;
  /**
   * The default the host works out when `CLAUDE_STREAM_IDLE_TIMEOUT_MS` is raised while the stream
   * watchdog is on ("the default rises with it"); `DEFAULT_STALL_MS` when absent. Only the default
   * rises: the call's `stallMs` and the environment variable still win over it.
   */
  readonly defaultMs?: number | undefined;
  readonly env: Readonly<Record<string, string | undefined>>;
}

/**
 * How long Claude Code waits for output from an agent before it ends the attempt: the call's own
 * `stallMs`, else `CLAUDE_ASYNC_AGENT_STALL_TIMEOUT_MS`, else the default (10 minutes, or the
 * host's `defaultMs`). A value that is not a positive whole number is no value (judgment).
 */
export const stallWindowMs = (input: StallWindowInput): number => call('stallWindowMs', input);

export interface SilenceInput {
  /** Milliseconds since the agent's last output. */
  readonly sinceOutputMs: number;
  /** Of those, the time spent waiting on its own tool calls. */
  readonly toolWaitMs: number;
  /** Of those, the time spent waiting for a usage-limit reset. */
  readonly usageLimitWaitMs: number;
  readonly windowMs: number;
}

/** "Time the agent spends waiting on its own tool calls or on a usage-limit reset doesn't count toward the stall window." */
export const hasStalled = (input: SilenceInput): boolean => call('hasStalled', input);

/** Where an `agent()` call sits in the script. */
export const CALL_SITES: readonly ['parallel', 'pipeline', 'awaited'] = call('CALL_SITES');
export type CallSite = (typeof CALL_SITES)[number];

export type AfterFailure =
  | { readonly runContinues: true; readonly result: null }
  | { readonly runContinues: false; readonly error: PluginIssue };

/**
 * Whether the run goes on after an `agent()` call failed: "Inside `parallel()` or `pipeline()`: the
 * run carries on with `null` in place of the agent's result. Awaited directly: the run ends with the
 * error."
 */
export const afterAgentFailure = (site: CallSite, failure: AgentFailure): AfterFailure =>
  call('afterAgentFailure', site, failure);
