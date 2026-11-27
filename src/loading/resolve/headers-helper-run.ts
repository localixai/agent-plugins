/**
 * Running a server's `headersHelper` and using what it prints (mcp.md, "Use dynamic headers for
 * custom authentication"). The kit starts nothing itself: the command goes through `CommandRunner`,
 * and the connection is the host's, handed in as a function.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/headers_helper_run.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { CommandRunner } from '../../ports/command-runner.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HelperRun } from './headers-helper.ts';

const call = coreModule('loading/resolve/headers-helper-run');

/**
 * Runs the planned command in a shell (`bash`; a host on Windows maps it to the shell it has, as for
 * a hook) from the planned directory and environment, under 10 seconds, and reads the JSON object
 * of string pairs it prints. Every call runs the command: nothing is cached. A command that fails
 * is a `PluginIssue`, never a throw; judgment (spec/judgments.md): the issue's server is the
 * `CLAUDE_CODE_MCP_SERVER_NAME` the helper got.
 */
export const runMcpHeadersHelper = (
  plan: HelperRun,
  runner: CommandRunner,
  { signal }: { readonly signal: AbortSignal },
): Promise<{ readonly headers: Record<string, string> } | PluginIssue> =>
  call.async('runMcpHeadersHelper', plan, runner, { signal });

export interface ConnectionHeaders {
  readonly headers: Record<string, string>;
  /**
   * Whether OAuth is still the fallback: "If you configure an `Authorization` header, Claude Code
   * uses it as the server's credential and doesn't fall back to OAuth", in `headers` or from the helper.
   */
  readonly oauthFallback: boolean;
}

/**
 * "Dynamic headers override any static `headers` with the same name": names compare without regard
 * to case (judgment, spec/judgments.md).
 */
export const connectionHeaders = (
  staticHeaders: Readonly<Record<string, string>>,
  dynamicHeaders: Readonly<Record<string, string>>,
): ConnectionHeaders => call('connectionHeaders', staticHeaders, dynamicHeaders);

/**
 * How an attempt ended. `needs-authentication`: the retry after a rejected call was rejected too.
 * `failed`: the helper failed, or the server rejected the helper's credential while connecting; the
 * issue then carries the errors.md message when the helper supplied `Authorization`.
 */
export interface HelperAttempt<T> {
  readonly outcome: 'ok' | 'needs-authentication' | 'failed';
  readonly response?: T;
  /** How many times the helper was run again after a rejection: at most one. */
  readonly reruns: number;
  readonly issue?: PluginIssue;
}

/** What one run of the helper gives: the headers to send, or why there are none. */
export type HelperOutput = { readonly headers: Record<string, string> } | PluginIssue;

/**
 * One call or one connection with a helper (mcp.md): `produce` runs the helper fresh and gives the
 * headers it printed (`runMcpHeadersHelper`'s output, or none where the plan skipped it), and `send`
 * makes the request with them, merging the server's static headers (`connectionHeaders`). `server` is
 * the name Claude Code shows, for the issue.
 *
 * - `call`: on a 401 or 403 the helper runs again once, the request is retried once with the fresh
 *   headers, and "Claude Code marks the server as needing authentication in `/mcp` only if that retry
 *   also fails".
 * - `connect`: "If the server rejects the helper's credential while connecting, Claude Code reports
 *   the connection as failed rather than marking the server as needing authentication"; no re-run
 *   (a reconnect from `/mcp` runs the helper again).
 */
export const attemptWithHeadersHelper = <T extends { readonly status: number }>(
  phase: 'connect' | 'call',
  produce: () => Promise<HelperOutput>,
  send: (headers: Record<string, string>) => Promise<T>,
  server: string,
): Promise<HelperAttempt<T>> =>
  call.async('attemptWithHeadersHelper', phase, produce, send, server);
