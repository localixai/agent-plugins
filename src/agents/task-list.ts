import { isFalsyEnv } from '../format/env-flag.ts';
import type { EffortLevel } from '../protocol/official.ts';

/**
 * What the client's task list (`/tasks`) shows of subagents (§5.8.2 "Checking", §5.12.3): each
 * subagent's row names the model it runs on and its effort level when one is set, and a row reopens
 * the transcripts of subagents that have left the list unless `CLAUDE_CODE_SUBAGENT_HISTORY=0`.
 * Drawing the list is the host's; the kit says what a row carries.
 */

export interface SubagentTaskRow {
  /** The model the subagent runs on: `resolveSubagentModel`, after `checkAgainstAllowlist`. */
  readonly model: string;
  /** The effort level set for the subagent (`resolveSubagentEffort`); absent when none is set. */
  readonly effort?: EffortLevel | number;
}

/** The row of one subagent: its model, and its effort level only when there is one. */
export function subagentTaskRow(input: {
  readonly model: string;
  readonly effort?: EffortLevel | number | undefined;
}): SubagentTaskRow {
  const { model, effort } = input;
  return effort === undefined ? { model } : { model, effort };
}

/**
 * Whether the task list offers the row from which the user reopens subagents that have ended:
 * `CLAUDE_CODE_SUBAGENT_HISTORY=0` removes it (any off spelling, as every on/off variable; J3.16).
 */
export const offersSubagentHistory = (value: string | undefined): boolean => !isFalsyEnv(value);
