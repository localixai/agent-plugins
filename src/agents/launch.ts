import type { CacheTtl } from '../format/agent-scalars.ts';
import type { AgentIsolation } from '../format/agent-fields.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Small pure decisions a host takes when it starts a subagent, one per frontmatter field that
 * changes how the run is set up (sub-agents.md, "Frontmatter reference", "Run subagents in
 * foreground or background", "What loads at startup", "Set the auto-compact window for one
 * subagent"). The kit decides; compacting, running the turn loop, creating a worktree and loading
 * files are the host's. The model and effort decisions are in `launch-model.ts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/launch.rs`, docs/RUST.md).
 */
const call = coreModule('agents/launch');

export interface AutoCompactInput {
  /** The definition's `autoCompactWindow` (already read; a dropped field is undefined). */
  readonly field?: number | undefined;
  /** The window the subagent would otherwise inherit. */
  readonly inherited: number;
  /** `CLAUDE_CODE_AUTO_COMPACT_WINDOW` as a number. */
  readonly env?: number | undefined;
  /** The agent runs the whole session (`--agent`, the `agent` setting). */
  readonly asMainSession: boolean;
}

/**
 * The auto-compact window a subagent compacts at. The variable wins ("the subagent uses that
 * variable's window instead and the field has no effect, unless Claude Code ignores the variable's
 * value"); the field only lowers ("To compact earlier, set a value below that window") and "has no
 * effect" for a session run as the agent.
 */
export const effectiveAutoCompactWindow = (input: AutoCompactInput): number =>
  call('effectiveAutoCompactWindow', input);

export interface TurnLimitOutcome {
  readonly stop: boolean;
  /** "Claude Code returns its output marked as partial" and Claude can resume it. */
  readonly partial: boolean;
}

/**
 * Whether the subagent has reached `maxTurns`: "Maximum number of agentic turns before the
 * subagent stops". No limit unless it is a positive whole number (judgment, spec/judgments.md).
 */
export const turnLimitOutcome = (
  maxTurns: number | undefined,
  turnsTaken: number,
): TurnLimitOutcome => call('turnLimitOutcome', maxTurns, turnsTaken);

/**
 * The `cacheTtl` the frontmatter contributes to a subagent's requests: it "ignores `1h` while your
 * Claude subscription is using usage credits" (prompt-caching.md, "Choose the TTL yourself"). The
 * result is the frontmatter's step of the precedence; `undefined` leaves the decision to the next one.
 */
export const effectiveCacheTtl = (input: {
  readonly cacheTtl?: CacheTtl | undefined;
  readonly usingUsageCredits: boolean;
}): CacheTtl | undefined => call('effectiveCacheTtl', input);

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
}

/** Fork mode: on by default, `CLAUDE_CODE_FORK_SUBAGENT=0` (any off spelling) turns it off (spec §5.12.3). */
export const forkModeOn = (value: string | undefined): boolean => call('forkModeOn', value);

/**
 * Whether a subagent runs in the background. "With nothing configured, a subagent that Claude
 * spawns from your main conversation starts in the background"; `background: true` keeps it there
 * "even when Claude asks to wait for it". With fork mode on, an interactive main conversation cannot
 * wait, so the subagent starts in the background there (env-vars.md, `CLAUDE_CODE_FORK_SUBAGENT`).
 */
export const runsInBackground = (input: BackgroundInput): boolean =>
  call('runsInBackground', input);

export interface ClaudeMdScopes {
  readonly user: boolean;
  readonly project: boolean;
  readonly local: boolean;
  readonly managedPolicy: boolean;
  /** A subdirectory's CLAUDE.md and matching path-scoped rules, loaded when the subagent works with files there. */
  readonly nested: boolean;
}

/**
 * The CLAUDE.md files a subagent starts with. `omitClaudeMd` drops the user, project and local
 * files; "managed policy files still load, except for managed subagents"; it is "Ignored when the
 * agent runs as the main session agent". A subdirectory's CLAUDE.md and matching path-scoped rules
 * "still load when the subagent works with files there", with or without `omitClaudeMd` (`nested`).
 */
export const claudeMdScopes = (input: {
  readonly omitClaudeMd?: boolean | undefined;
  readonly asMainSession: boolean;
  readonly managedSubagent: boolean;
}): ClaudeMdScopes => call('claudeMdScopes', input);

export type WorktreeDisposition = 'none' | 'cleanup' | 'keep';

/**
 * What happens to a subagent's worktree when it finishes. Only `isolation: worktree` has one
 * (sub-agents.md, `isolation`); it is "automatically cleaned up if the subagent makes no changes",
 * and one with changes "stays on disk until the periodic sweep" (worktrees.md, "Isolate subagents
 * with worktrees"). Without `isolation` there is no worktree: `none`.
 */
export const worktreeDisposition = (input: {
  readonly isolation?: AgentIsolation | undefined;
  readonly hasChanges: boolean;
}): WorktreeDisposition => call('worktreeDisposition', input);

/**
 * The first user turn to submit for an agent run as the main session agent: its `initialPrompt`,
 * "Prepended to any user-provided prompt". A subagent run submits nothing (undefined). Processing
 * commands and skills in the text is the host's.
 */
export const mainSessionFirstTurn = (input: {
  readonly initialPrompt?: string | undefined;
  readonly asMainSession: boolean;
  readonly userPrompt?: string | undefined;
}): string | undefined => call('mainSessionFirstTurn', input);
