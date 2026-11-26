import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The runtime constraints of a workflow run (workflows.md, "Behavior and limits"). Pure decisions over
 * counts the host holds: the kit runs no workflow and keeps no run state.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/workflow/limits.rs`, docs/RUST.md).
 */
const call = coreModule('agents/workflow/limits');

/** The calls that take a list of work (workflows.md, "What the saved script looks like"). */
export const FAN_OUT_CALLS: readonly ['parallel', 'pipeline'] = call('FAN_OUT_CALLS');
export type FanOutCall = (typeof FAN_OUT_CALLS)[number];

export type Admission =
  | { readonly admitted: true }
  | { readonly admitted: false; readonly issue: PluginIssue };

/**
 * Whether the run may start one more agent, given how many `agent()` calls it has started. The
 * 1,000th is the last; the next call fails with an error (judgment: the docs name the cap and its
 * purpose, not the failure; a restart of an agent after a stall is the same agent and is not counted).
 */
export const admitAgent = (started: number): Admission => call('admitAgent', started);

/** "the runtime rejects a longer list with an error": nothing in the list runs when it is too long. */
export const checkFanOut = (call_: FanOutCall, items: number): PluginIssue | undefined =>
  call('checkFanOut', call_, items);

/**
 * What can interrupt a run. "A run pauses on its own only for agent permission prompts and a
 * usage-limit wait": nothing else pauses it. `script-question` and `stage-sign-off` are the kit's own
 * names for the two kinds of mid-run input the docs turn away ("For sign-off between stages, run each
 * stage as its own workflow"), so a host can name why it refuses (spec/judgments.md).
 */
export const INTERRUPTIONS: readonly [
  'agent-permission-prompt',
  'usage-limit-wait',
  'script-question',
  'stage-sign-off',
] = call('INTERRUPTIONS');
export type Interruption = (typeof INTERRUPTIONS)[number];

export type Pause =
  | { readonly pauses: true }
  | { readonly pauses: false; readonly issue: PluginIssue };

/** Whether the run waits for it, or refuses it (judgment: the docs give no wording for the refusal). */
export const interruptRun = (cause: Interruption): Pause => call('interruptRun', cause);

/**
 * How many agents a run executes at once: 16, "fewer when Claude Code has fewer CPUs available", or the
 * variable's value from 1 to 256. It "takes plain digits only; out-of-range values and other spellings
 * keep the default". Whether the CPU count also caps an explicit value is not said: it does not
 * (spec/judgments.md).
 */
export const workflowConcurrency = (
  env: Readonly<Record<string, string | undefined>>,
  cpus: number,
): number => call('workflowConcurrency', env, cpus);
