import { positiveWholeEnv } from '../../../format/env-number.ts';
import type { Clock, Environment, McpCaller } from '../../../ports/index.ts';
import type { HookEvent } from '../../../protocol/official.ts';
import type { HandlerEntry, HandlerResult, RawRun, RunContext } from '../context.ts';
import { handlerTimeoutMs } from '../timeouts.ts';
import { failureRun, remotePath, withoutPort } from './failure-run.ts';
import { substituteInput } from './mcp-input.ts';
import type { McpRun, McpUnreadable } from './types.ts';

/**
 * An `mcp_tool` hook (hooks.md, "MCP tool hook fields"). The `server` is passed as the hook writes
 * it: for a plugin-bundled server that is "the scoped name `plugin:<plugin-name>:<server-name>`,
 * not the bare server key". Running no hook on `SessionStart` at launch and on `Setup` (no MCP
 * client yet) is the select stage's (`select/mcp-client.ts`).
 */

/**
 * "On events where a hook can block or change the result, such as `PreToolUse` or `Stop`, Claude
 * Code waits for a connecting server ... On observational events, such as `Notification` or
 * `SessionEnd`, it doesn't wait."
 *
 * Judgment (spec/judgments.md): the docs name four events, so the kit sorts the rest by the same
 * test. An event waits when exit code 2 can stop what it guards ("Can block?: Yes" in "Exit code 2
 * behavior per event") or its output decides a result: `PermissionRequest` (`decision`),
 * `PermissionDenied` (`retry`), `PostToolUse` (`updatedToolOutput`, `decision`), `MessageDisplay`
 * (`displayContent`). An event whose hooks only add context or watch (`SessionStart`,
 * `SubagentStart`, `PostToolUseFailure`) does not.
 */
export const WAITS_FOR_SERVER: { readonly [E in HookEvent]: boolean } = {
  PreToolUse: true,
  PermissionRequest: true,
  PermissionDenied: true,
  PostToolUse: true,
  PostToolBatch: true,
  UserPromptSubmit: true,
  UserPromptExpansion: true,
  Stop: true,
  SubagentStop: true,
  TeammateIdle: true,
  TaskCreated: true,
  TaskCompleted: true,
  ConfigChange: true,
  PreCompact: true,
  PreModelSwitch: true,
  Elicitation: true,
  ElicitationResult: true,
  WorktreeCreate: true,
  WorktreeRemove: true,
  MessageDisplay: true,
  SessionStart: false,
  Setup: false,
  SessionEnd: false,
  StopFailure: false,
  PostToolUseFailure: false,
  SubagentStart: false,
  Notification: false,
  InstructionsLoaded: false,
  CwdChanged: false,
  DirectoryAdded: false,
  FileChanged: false,
  PostCompact: false,
  PostModelSwitch: false,
};

/** `MCP_TIMEOUT`: "Timeout in milliseconds for MCP server startup (default: 30000, or 30 seconds)". */
const MCP_TIMEOUT_VARIABLE = 'MCP_TIMEOUT';
const DEFAULT_MCP_TIMEOUT_MS = 30_000;

function mcpTimeoutMs(environment: Environment): number {
  // J3.17: read as every numeric variable is (spec §3.6.1 item 2).
  return positiveWholeEnv(environment.get(MCP_TIMEOUT_VARIABLE)) ?? DEFAULT_MCP_TIMEOUT_MS;
}

/** "For at most `MCP_TIMEOUT` and within the hook's own `timeout`"; none on an observational event. */
function waitMs(entry: HandlerEntry<'mcp_tool'>, context: RunContext, hookMs: number | undefined) {
  if (!WAITS_FOR_SERVER[entry.event]) return 0;
  const limit = mcpTimeoutMs(context.ports.environment);
  return hookMs === undefined ? limit : Math.min(limit, hookMs);
}

/** The hook's `timeout` (none: unlimited) and the clock reading when the hook started. */
interface Budget {
  readonly hookMs: number | undefined;
  readonly startedAt: number;
}

/**
 * What ends the hook before the tool is called, or `undefined` when the call goes ahead: a session
 * abort during the wait ("aborted" is the shared `cancelled` run; the port can only answer
 * `unavailable`), a server that is not connected ("the hook produces a non-blocking error"), or a
 * wait that used up the hook's `timeout`.
 */
async function endedBeforeCall(
  entry: HandlerEntry<'mcp_tool'>,
  context: RunContext,
  client: McpCaller,
  budget: Budget,
): Promise<RawRun | undefined> {
  const { server, tool } = entry.handler;
  const { hookMs, startedAt } = budget;
  const availability = await client.ensureConnected(
    server,
    waitMs(entry, context, hookMs),
    context.signal,
  );
  if (context.signal.aborted) return { kind: 'cancelled' };
  if (availability === 'unavailable') {
    const text = `The MCP server "${server}" is not connected, so the hook did not call ${tool}.`;
    return { kind: 'mcp', isError: true, text };
  }
  const left = leftMs(hookMs, startedAt, context.ports.clock);
  return left !== undefined && left <= 0 ? { kind: 'timed-out' } : undefined;
}

/**
 * What is left of the hook's `timeout` for the call, "within the hook's own `timeout`" being the
 * wait and the call together. Judgment (spec/judgments.md): a host with no `clock` leaves the call
 * the whole `timeout`, so the wait may add to it.
 */
function leftMs(hookMs: number | undefined, startedAt: number, clock: Clock | undefined) {
  return hookMs === undefined || clock === undefined ? hookMs : hookMs - (clock.now() - startedAt);
}

export async function runMcpTool(
  entry: HandlerEntry<'mcp_tool'>,
  context: RunContext,
): Promise<HandlerResult> {
  const client: McpCaller | undefined = context.ports.mcp;
  if (client === undefined) return withoutPort(entry, 'mcp_tool', 'McpCaller');
  const { server, tool } = entry.handler;
  const hookMs = handlerTimeoutMs(entry, context.timeouts);
  const budget = { hookMs, startedAt: context.ports.clock?.now() ?? 0 };
  const ended = await endedBeforeCall(entry, context, client, budget);
  if (ended !== undefined) return { run: ended, issues: [] };
  const args = substituteInput(entry.handler.input, context.input, remotePath(entry, 'input'));
  const timeoutMs = leftMs(hookMs, budget.startedAt, context.ports.clock);
  const answer = await client.callTool({
    server,
    tool,
    input: args.input,
    ...(timeoutMs === undefined ? {} : { timeoutMs }),
    signal: context.signal,
  });
  const run =
    'failure' in answer
      ? (unreadableRun(answer.failure, server, tool) ??
        failureRun(answer, (text) => ({ kind: 'mcp', isError: true, text })))
      : ({ kind: 'mcp', isError: answer.isError, text: answer.text } as const);
  return { run, issues: args.issues };
}

const SMALLER = 'If this keeps happening, the tool must send a smaller answer.';

/** The hook errors of an answer the client could not read (hooks.md, "How the tool's result is read"). */
const UNREADABLE: { readonly [U in McpUnreadable]: (server: string, tool: string) => string } = {
  'message-too-large': (server, tool) =>
    `MCP tool "${tool}" on server "${server}" gave no answer that could be read, because a message from the server was too large, so this hook gave no verdict. ${SMALLER}`,
  'missing-text': (server, tool) =>
    `MCP tool "${tool}" on server "${server}" answered without its text, which happens when a result is too large, so this hook gave no verdict. ${SMALLER}`,
};

const isUnreadable = (failure: string): failure is McpUnreadable =>
  Object.hasOwn(UNREADABLE, failure);

/** A port failure that says the answer could not be read, as the run the interpreter refuses on. */
function unreadableRun(failure: string, server: string, tool: string): McpRun | undefined {
  if (!isUnreadable(failure)) return undefined;
  return {
    kind: 'mcp',
    isError: true,
    unreadable: failure,
    text: UNREADABLE[failure](server, tool),
  };
}
