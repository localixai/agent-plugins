import type { PluginIssue } from '../format/issues.ts';
import type { HookEvent } from '../protocol/official.ts';
import type { EventFactsOf, HookSession } from './contracts.ts';
import type { UserInterrupt } from './interrupt.ts';
import type {
  BackgroundDelivery,
  BlockInfo,
  EventEffects,
  Feedback,
  HandlerStatus,
  BlockVia,
  HookNotice,
  RunFacts,
} from './verdict/types.ts';

/**
 * A record of one handler's run for the host's log: what it came to, and the facts the debug line
 * is written from (`RunFacts`, Agent Plugins Extended §7.4.12 item 2).
 */
export interface HandlerRun extends RunFacts {
  readonly handlerId: string;
  readonly status: HandlerStatus;
  readonly via?: BlockVia;
}

export interface DispatchRequest<E extends HookEvent> {
  readonly event: E;
  readonly facts: EventFactsOf<E>;
  readonly session: HookSession;
  readonly signal: AbortSignal;
  /** The key the user pressed while a tool ran (`PostToolUse` only: docs/API.md, "How a run is read"). */
  readonly interrupt?: UserInterrupt;
  /**
   * `TaskCompleted` only: what fired it (hooks.md, "TaskCompleted"): `task-update`, an agent marking
   * a task completed through the `TaskUpdate` tool, or `teammate-turn`, a teammate finishing its turn
   * with in-progress tasks. The input does not say; without it `effects.stopCondition` tells the host
   * where `continue: false` holds.
   */
  readonly taskCompletedBy?: 'task-update' | 'teammate-turn';
  /**
   * `PostToolUse` only: the call failed, and `facts.tool_response` is its error text. Only the hooks
   * of managed settings run (hooks.md, "PostToolUse", 2.1.330: they "also run after some failed
   * calls"; Agent Plugins Extended J7.215). Which failed calls is the host's; `PostToolUseFailure`
   * still fires for the same failure.
   */
  readonly failedCall?: boolean;
  /**
   * `PreToolUse` only: Code Mode's code made the call (code-mode.md, "Tool call refused by a hook";
   * Agent Plugins Extended §7.6.11.4, J7.213). A hook that times out, or an agent hook that ends
   * without a decision, refuses the call with `the hook did not finish` unless `onFailure` is set.
   * `UserPromptSubmit`: the prompt is the output of Code Mode's code (before 2.1.328 it reached
   * these hooks with `source` `poll_event`). No `UserPromptSubmit` hook runs on it, managed ones
   * included, and the outcome lets it through (code-mode.md, "Move a check from a
   * `UserPromptSubmit` hook"; Agent Plugins Extended §7.6.11.3 item 4).
   */
  readonly fromCode?: boolean;
  /**
   * `SubagentStart` and `SubagentStop` only: the subagent is the built-in `web-fetch` agent (not a
   * custom agent of that name, which replaces it). Only the hooks of managed settings run; the
   * `PreToolUse` and `PostToolUse` hooks matching `WebFetch` still run for its fetches, so the flag
   * changes nothing on the other events (Agent Plugins Extended §7.5.18, §7.5.19, §7.6.3 item 8,
   * §5.12.1).
   */
  readonly webFetchAgent?: boolean;
  /**
   * `PostToolUse` only: the session is a Cowork cloud session. A hook's `updatedToolOutput` for
   * `AskUserQuestion` or `ExitPlanMode` is then ignored, and Claude reads the original answer or
   * plan (Agent Plugins Extended §7.5.13); other tools keep the replacement.
   */
  readonly coworkCloud?: boolean;
}

/** What an event came to, merged over every handler that ran. */
export interface HookOutcome<E extends HookEvent> {
  readonly event: E;
  /** `false`: the guarded action must not happen. */
  readonly proceed: boolean;
  readonly block?: BlockInfo & { readonly by: string };
  /**
   * `UserPromptSubmit`: the user interrupted while the hooks checked the prompt. The prompt did not
   * reach Claude (`proceed` is `false`); `restoreInput` says the text goes back to the input, which
   * it does not when a hook blocked the prompt at the same moment.
   */
  readonly interrupted?: { readonly restoreInput: boolean };
  readonly continue: boolean;
  readonly stopReason?: string;
  readonly systemMessages: readonly string[];
  readonly additionalContext: readonly string[];
  readonly effects: EventEffects;
  readonly terminalSequences: readonly string[];
  readonly notices: readonly HookNotice[];
  readonly feedback: readonly Feedback[];
  /** The stdout of each `WorktreeCreate` command hook that exited 0, in handler order. */
  readonly stdout: readonly string[];
  readonly runs: readonly HandlerRun[];
  /** Ids the session removes: skill-frontmatter `once: true` hooks that ran successfully. */
  readonly consumedOnce: readonly string[];
  readonly background: readonly Promise<BackgroundDelivery>[];
  readonly issues: readonly PluginIssue[];
}
