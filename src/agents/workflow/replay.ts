/**
 * Which agents run again when a workflow run is relaunched (workflows.md, "Resume after a pause",
 * "Which agents run again"): "Claude Code replays the run in the order agents start, and each agent
 * either returns its saved result or runs again". Pure over the saved record of the earlier run and the
 * calls the script starts now, one at a time, because a rerun agent may return something else and
 * change the prompts after it. A call is its prompt and its options: the SDK's `resumeFromRunId` says
 * "Completed agent() calls with unchanged (prompt, opts) return their cached results".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/workflow/replay.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('agents/workflow/replay');

export const SAVED_STATUSES: readonly ['completed', 'failed', 'running'] = call('SAVED_STATUSES');
export type SavedStatus = (typeof SAVED_STATUSES)[number];

/** One agent of the earlier run, in the order it started. */
export interface SavedAgent {
  readonly prompt: string;
  /**
   * A fingerprint of the call's other options (`schema`, `label`, `model` and the rest of `opts`),
   * which the host computes the same way for the saved record and for a call it starts now; absent
   * when the host keeps none, then only the prompt is compared (judgment).
   */
  readonly options?: string;
  readonly status: SavedStatus;
  /** Its result, for a completed agent. */
  readonly result?: unknown;
}

/** An `agent()` call the script starts now: its prompt and the fingerprint of its options. */
export interface AgentCall {
  readonly prompt: string;
  readonly options?: string;
}

/** Why an agent runs again. */
export const RERUN_CAUSES: readonly [
  'prompt-differs',
  'options-differ',
  'after-change',
  'failed',
  'after-failed',
  'was-running',
  'not-saved',
] = call('RERUN_CAUSES');
export type RerunCause = (typeof RERUN_CAUSES)[number];

export type Replay =
  | { readonly saved: true; readonly result: unknown }
  | { readonly saved: false; readonly cause: RerunCause };

/** Where the replay is: the next saved agent, and whether everything from here runs again, and why. */
export interface ReplayState {
  readonly next: number;
  readonly cascade: 'after-change' | 'after-failed' | undefined;
}

export const REPLAY_START: ReplayState = call('REPLAY_START');

/** What the agent the script starts now does: returns its saved result or runs again. */
export const replayAgent = (
  saved: readonly SavedAgent[],
  state: ReplayState,
  call_: AgentCall,
): { readonly replay: Replay; readonly state: ReplayState } =>
  call('replayAgent', saved, state, call_);

/**
 * The decisions for a script that starts `calls` in this order, when no rerun agent returns anything
 * different from before (what a relaunch of an unchanged or edited-prompt script shows).
 */
export const planReplay = (saved: readonly SavedAgent[], calls: readonly AgentCall[]): Replay[] =>
  call('planReplay', saved, calls);

export const STOP_SCOPES: readonly ['whole-run', 'one-agent'] = call('STOP_SCOPES');
export type StopScope = (typeof STOP_SCOPES)[number];

/**
 * How the saved record keeps an agent that had not finished when it was stopped: "Stopping the whole
 * run doesn't count any agent as failed"; "Stopping one agent alone, by selecting it in `/workflows`
 * and pressing `x`, counts as failing."
 */
export const RECORDED_WHEN_STOPPED: { readonly [S in StopScope]: SavedStatus } =
  call('RECORDED_WHEN_STOPPED');
