/**
 * The `ttlMs` and `pollIntervalMs` an MCP server SHOULD send in a task answer and every `tasks/get`
 * response (mcp.md, "Choose a time to live and a poll interval"; §8.10): `ttlMs` is how long the
 * server keeps the task from its creation, its longest run time plus the time it keeps the result
 * (at least an hour, so a user who exits and resumes the session still gets it), or `null` for no
 * limit; `pollIntervalMs` is 10000 to 30000 for a job longer than a minute and 2000 for a shorter one.
 * For a plugin author's server or its tests; the client polls as the server says.
 */

/** "At least an hour": how long a finished task's result is kept by default. */
export const MCP_TASK_MIN_RESULT_KEEP_MS = 60 * 60 * 1000;

const MINUTE_MS = 60 * 1000;
const LONG_JOB_POLL = { min: 10_000, max: 30_000 } as const;
const SHORT_JOB_POLL = { min: 2000, max: 2000 } as const;

export interface McpTaskTimingInput {
  /** The longest the job runs, in milliseconds. */
  readonly longestRunMs: number;
  /** How long the result is kept after the job ends; at least an hour is what the docs advise. */
  readonly keepResultMs?: number;
  /** The server keeps tasks with no limit: `ttlMs` is `null`. */
  readonly unlimited?: boolean;
}

export interface McpTaskTiming {
  readonly ttlMs: number | null;
  /** The interval range to send: one value inside it. */
  readonly pollIntervalMs: { readonly min: number; readonly max: number };
}

/** What a server SHOULD send for a job: a 30-minute job kept one hour has `ttlMs` 5400000. */
export function recommendedMcpTaskTiming(input: McpTaskTimingInput): McpTaskTiming {
  const keep = Math.max(
    input.keepResultMs ?? MCP_TASK_MIN_RESULT_KEEP_MS,
    MCP_TASK_MIN_RESULT_KEEP_MS,
  );
  return {
    ttlMs: input.unlimited === true ? null : input.longestRunMs + keep,
    pollIntervalMs: input.longestRunMs > MINUTE_MS ? LONG_JOB_POLL : SHORT_JOB_POLL,
  };
}
