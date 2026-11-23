import { isFalsyEnv } from '../format/env-flag.ts';
import { AUTO_COMPACT_MIN } from '../format/agent-scalars.ts';
import type { CacheTtl } from '../format/agent-scalars.ts';
import type { AgentIsolation } from '../format/agent-fields.ts';

/**
 * Small pure decisions a host takes when it starts a subagent, one per frontmatter field that
 * changes how the run is set up (sub-agents.md, "Frontmatter reference", "Run subagents in
 * foreground or background", "What loads at startup", "Set the auto-compact window for one
 * subagent"). The kit decides; compacting, running the turn loop, creating a worktree and loading
 * files are the host's. The model and effort decisions are in `launch-model.ts`.
 */

/** model-config.md: `CLAUDE_CODE_AUTO_COMPACT_WINDOW` from 100000 to 1000000; outside, the limit. */
const VARIABLE_MIN = 100_000;
const VARIABLE_MAX = 1_000_000;

export interface AutoCompactInput {
  /** The definition's `autoCompactWindow` (already read; a dropped field is undefined). */
  readonly field?: number | undefined;
  /** The window the subagent would otherwise inherit. */
  readonly inherited: number;
  /** `CLAUDE_CODE_AUTO_COMPACT_WINDOW` as a number. */
  readonly env?: number | undefined;
  /** The context window of the subagent's model; absent, `inherited` stands for it. */
  readonly modelWindow?: number | undefined;
  /** The agent runs the whole session (`--agent`, the `agent` setting). */
  readonly asMainSession: boolean;
}

const clamp = (tokens: number): number => Math.min(Math.max(tokens, VARIABLE_MIN), VARIABLE_MAX);

/** model-config.md: a value below the minimum or above the maximum is treated as the limit. */
const usableEnv = (env: number | undefined): number | undefined =>
  env !== undefined && Number.isFinite(env) && env > 0 ? clamp(env) : undefined;

/** A field the subagent applies: a whole number of 100000 or more, no upper bound (§5.5). */
const usableField = (input: AutoCompactInput): number | undefined => {
  const { field } = input;
  if (input.asMainSession || field === undefined) return undefined;
  return Number.isSafeInteger(field) && field >= AUTO_COMPACT_MIN ? field : undefined;
};

/**
 * The auto-compact window a subagent compacts at (§5.10). The field only lowers ("a value not below
 * the window the subagent would otherwise inherit changes nothing") and has no effect for a session
 * run as the agent. With `CLAUDE_CODE_AUTO_COMPACT_WINDOW` also set, the subagent uses the lowest of
 * the field's value, its model's context window and the variable's window.
 */
export function effectiveAutoCompactWindow(input: AutoCompactInput): number {
  const field = usableField(input);
  const env = usableEnv(input.env);
  const windows =
    env === undefined ? [input.inherited] : [input.modelWindow ?? input.inherited, env];
  return Math.min(...windows, ...(field === undefined ? [] : [field]));
}

export interface TurnLimitOutcome {
  readonly stop: boolean;
  /** "Claude Code returns its output marked as partial" and Claude can resume it. */
  readonly partial: boolean;
}

/**
 * Whether the subagent has reached `maxTurns`: "Maximum number of agentic turns before the
 * subagent stops". No limit unless it is a positive whole number (judgment, spec/judgments.md).
 */
export function turnLimitOutcome(
  maxTurns: number | undefined,
  turnsTaken: number,
): TurnLimitOutcome {
  const reached =
    maxTurns !== undefined && Number.isInteger(maxTurns) && maxTurns >= 1 && turnsTaken >= maxTurns;
  return { stop: reached, partial: reached };
}

const LONG_TTL: CacheTtl = '1h';

/**
 * The `cacheTtl` the frontmatter contributes to a subagent's requests: it "ignores `1h` while your
 * Claude subscription is using usage credits" (prompt-caching.md, "Choose the TTL yourself"). The
 * result is the frontmatter's step of the precedence; `undefined` leaves the decision to the next one.
 */
export function effectiveCacheTtl(input: {
  readonly cacheTtl?: CacheTtl | undefined;
  readonly usingUsageCredits: boolean;
}): CacheTtl | undefined {
  const { cacheTtl, usingUsageCredits } = input;
  return usingUsageCredits && cacheTtl === LONG_TTL ? undefined : cacheTtl;
}

export interface BackgroundInput {
  /** The definition's `background`. */
  readonly background?: boolean | undefined;
  /** Claude asked to wait for the result (on its judgment or yours in `claude -p` and the SDK; fork mode off). */
  readonly claudeWillWait: boolean;
  /** `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1`: "Claude waits for subagents to finish". */
  readonly backgroundDisabled?: boolean | undefined;
  /**
   * The main conversation is an interactive terminal, VS Code or desktop session on the user's
   * machine (not `-p` or the Agent SDK): with fork mode on it cannot wait for a subagent.
   */
  readonly interactive?: boolean | undefined;
  /** `CLAUDE_CODE_FORK_SUBAGENT` as set: fork mode is on unless it is an off spelling (`0`). */
  readonly forkSubagent?: string | undefined;
  /** The subagent is the built-in `web-fetch` agent, which always runs in the foreground (§5.12.3, §5.12.4). */
  readonly webFetchAgent?: boolean | undefined;
}

/** Fork mode: on by default, `CLAUDE_CODE_FORK_SUBAGENT=0` (any off spelling) turns it off (spec §5.12.3). */
export const forkModeOn = (value: string | undefined): boolean => !isFalsyEnv(value);

/**
 * Whether a subagent runs in the background. "With nothing configured, a subagent that Claude
 * spawns from your main conversation starts in the background"; `background: true` keeps it there
 * "even when Claude asks to wait for it". With fork mode on, an interactive main conversation cannot
 * wait, so the subagent starts in the background there (env-vars.md, `CLAUDE_CODE_FORK_SUBAGENT`).
 */
export function runsInBackground(input: BackgroundInput): boolean {
  if (input.backgroundDisabled === true || input.webFetchAgent === true) return false;
  const cannotWait = input.interactive === true && forkModeOn(input.forkSubagent);
  return input.background === true || cannotWait || !input.claudeWillWait;
}

/**
 * Whether a spawning subagent waits for a nested subagent's result (§5.12.2, "Let subagents spawn
 * their own subagents"): in an interactive session it always does; in `-p` and the SDK only for a
 * nested subagent in the foreground, and a background one reports to the main conversation if it
 * outlasts the subagent that spawned it.
 */
export function spawnerWaitsForNested(input: {
  readonly interactive: boolean;
  readonly nestedInBackground: boolean;
}): boolean {
  return input.interactive || !input.nestedInBackground;
}

export interface ClaudeMdScopes {
  readonly user: boolean;
  readonly project: boolean;
  readonly local: boolean;
  readonly managedPolicy: boolean;
  /** A subdirectory's CLAUDE.md and matching path-scoped rules, loaded when the subagent works with files there. */
  readonly nested: boolean;
}

const ALL_CLAUDE_MD: ClaudeMdScopes = {
  user: true,
  project: true,
  local: true,
  managedPolicy: true,
  nested: true,
};

/**
 * The CLAUDE.md files a subagent starts with. `omitClaudeMd` drops the user, project and local
 * files; "managed policy files still load, except for managed subagents"; it is "Ignored when the
 * agent runs as the main session agent". A subdirectory's CLAUDE.md and matching path-scoped rules
 * "still load when the subagent works with files there", with or without `omitClaudeMd` (`nested`).
 */
export function claudeMdScopes(input: {
  readonly omitClaudeMd?: boolean | undefined;
  readonly asMainSession: boolean;
  readonly managedSubagent: boolean;
}): ClaudeMdScopes {
  if (input.omitClaudeMd !== true || input.asMainSession) return ALL_CLAUDE_MD;
  return {
    user: false,
    project: false,
    local: false,
    managedPolicy: !input.managedSubagent,
    nested: true,
  };
}

export type WorktreeDisposition = 'none' | 'cleanup' | 'keep';

/**
 * What happens to a subagent's worktree when it finishes. Only `isolation: worktree` has one
 * (sub-agents.md, `isolation`); it is "automatically cleaned up if the subagent makes no changes",
 * and one with changes "stays on disk until the periodic sweep" (worktrees.md, "Isolate subagents
 * with worktrees"). Without `isolation` there is no worktree: `none`.
 */
export function worktreeDisposition(input: {
  readonly isolation?: AgentIsolation | undefined;
  readonly hasChanges: boolean;
}): WorktreeDisposition {
  if (input.isolation !== 'worktree') return 'none';
  return input.hasChanges ? 'keep' : 'cleanup';
}

/** Judgment: the docs say "Prepended to any user-provided prompt" and no separator; a blank line. */
const PROMPT_SEPARATOR = '\n\n';

/**
 * The first user turn to submit for an agent run as the main session agent: its `initialPrompt`,
 * "Prepended to any user-provided prompt". A subagent run submits nothing (undefined). Processing
 * commands and skills in the text is the host's.
 */
export function mainSessionFirstTurn(input: {
  readonly initialPrompt?: string | undefined;
  readonly asMainSession: boolean;
  readonly userPrompt?: string | undefined;
}): string | undefined {
  const initial = input.initialPrompt?.trim();
  if (!input.asMainSession || initial === undefined || initial === '') return undefined;
  const user = input.userPrompt?.trim();
  return user === undefined || user === '' ? initial : `${initial}${PROMPT_SEPARATOR}${user}`;
}
