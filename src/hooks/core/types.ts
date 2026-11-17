import type { CoreHookEvent } from '../../format/core/hooks.ts';

/**
 * What the core side of one hook occurrence is made of (Agent Plugins Extended §18.1.2): the core
 * input a core handler receives, the facts of a call only the host knows, and what each core
 * handler's run came to. Core §7.3 is cited as `core §n`.
 */

/**
 * What the host knows of a call that Claude Code's input does not carry, for the core tool profiles
 * (Extended §18.2.7). Each is optional: without it the profile that needs it is omitted.
 */
export interface CoreCallFacts {
  /** A shell command's exit code as the host observed it (`BashOutput` has none). */
  readonly exitCode?: number;
  /** The tool name an MCP server declared, as sent in `tools/call` (`latest_tag`). */
  readonly mcpToolName?: string;
  /** The `CallToolResult` the MCP server returned, before any truncation or replacement. */
  readonly mcpResult?: Readonly<Record<string, unknown>>;
  /** The text Claude received from a `Read` call (line numbers included; a notebook's cells). */
  readonly readText?: string;
  /**
   * `UserPromptSubmit`: who started the turn (J18.7). `external`: a submission from outside the
   * session's own work (the user, an SDK or `-p` caller, the poll-event channel, a message another
   * session or a channel sends), which fires the core event; `client`: a turn the host started
   * itself (a scheduled task, a `/loop` iteration, a task notification, an automatic
   * continuation), which does not. Required for every turn whose `source` is `system`, which holds
   * both kinds, whenever a core package has a `UserPromptSubmit` handler: the dispatch throws
   * without it. Absent otherwise, the `source` decides and a turn without one is external.
   */
  readonly turnOrigin?: 'external' | 'client';
}

/** A portable tool profile: its name, its input projection and, after the call, its result. */
export interface CoreToolProfile {
  readonly name: string;
  readonly input: Readonly<Record<string, unknown>>;
  readonly result?: Readonly<Record<string, unknown>>;
}

/** The core input of one occurrence and the event it is, with the tool name for notices. */
export interface CoreOccurrence {
  readonly event: CoreHookEvent;
  readonly input: Readonly<Record<string, unknown>>;
  readonly toolName?: string;
}

/** A successful core result: added context, or a decision with its reason (core §7.3.4–§7.3.8). */
export type CoreResult =
  | { readonly context: string }
  | { readonly decision: 'allow' | 'deny' | 'continue'; readonly reason?: string };

/** One core handler's run: its result, or why it failed (core §7.3.2 item 5). */
export interface CoreRun {
  /** `core:<plugin id>:<Event>:<index>`. */
  readonly handlerId: string;
  readonly pluginId: string;
  /** The command and its arguments as they ran, for the notice and the debug line. */
  readonly label: string;
  /** Absent with a failure, and when the handler expressed no opinion. */
  readonly result?: CoreResult;
  readonly failure?: string;
  readonly timedOut?: boolean;
  readonly exitCode?: number;
  readonly timeoutMs: number;
}
