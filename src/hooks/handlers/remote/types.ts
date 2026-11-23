/**
 * What the remote handler types report (hooks.md, "HTTP hook fields", "MCP tool hook fields",
 * "Prompt-based hooks", "Agent-based hooks"). A run says what happened and never interprets it:
 * what a status, an `isError` or an `ok: false` does on an event is the interpreter's. A timeout, a
 * cancellation and a hook that did not run are the shared `timed-out`, `cancelled` and `skipped`
 * runs of `context.ts`, so every handler type is timed out and skipped the same way.
 */

/** The server answered: any status, with the body as text. */
export interface HttpRun {
  readonly kind: 'http';
  readonly status: number;
  readonly body: string;
}

/** The request never got an answer ("Connection failure: non-blocking error"). */
export interface HttpFailedRun {
  readonly kind: 'http-failed';
  readonly message: string;
}

/** The MCP tool answered: `isError` is the tool's own flag, `text` its text content. */
export interface McpRun {
  readonly kind: 'mcp';
  readonly isError: boolean;
  readonly text: string;
  /**
   * The client could not read the answer (hooks.md, "How the tool's result is read", 2.1.330): the
   * server's message was too large, or the result came without its text. `text` is then the hook
   * error, and the hook gives no verdict.
   */
  readonly unreadable?: McpUnreadable;
}

/** Why an `mcp_tool` hook's answer could not be read: the port failure kinds of `McpCaller.callTool`. */
export type McpUnreadable = 'message-too-large' | 'missing-text';

/** The model's verdict, validated against the response schema (hooks.md, "Response schema"). */
export interface ModelRun {
  readonly kind: 'model';
  readonly ok: boolean;
  readonly reason?: string;
  /** Prompt hooks only: the model judged the condition can never be satisfied. */
  readonly impossible?: boolean;
}

/** The model did not answer, or answered something that is not the response schema. */
export interface ModelFailedRun {
  readonly kind: 'model-failed';
  readonly message: string;
}

export type RemoteRawRun = HttpRun | HttpFailedRun | McpRun | ModelRun | ModelFailedRun;
