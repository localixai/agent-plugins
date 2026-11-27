import { coreModule } from '../wasm/bridge.ts';
import type { McpTransport } from './mcp-config.ts';
import type { EnvLookup } from './mcp-env.ts';

/**
 * The environment variables that tune MCP servers and what each decides (env-vars.md, "Variables";
 * mcp.md, "Time limits on MCP tool calls", "Tool search for MCP server authors", "Option 3: Add a
 * local stdio server"). Pure over a lookup of the environment: reading the real one, and starting
 * anything, are the host's. The typed registry of every host variable lives with the environment
 * module; these are the semantics of the MCP rows.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_knobs.rs`, docs/RUST.md).
 */
const call = coreModule('format/mcp-knobs');

/** A positive whole number of a numeric variable (spec §3.6.1 item 2, J3.17); anything else is `undefined`. */
export const positiveWhole = (text: string | undefined): number | undefined =>
  call('positiveWhole', text);

export interface McpTimeouts {
  /** `MCP_TIMEOUT`: one server's connect attempt. */
  readonly startupMs: number;
  /** The wall-clock limit of one tool call. */
  readonly toolMs: number;
  /** The per-request timer of an HTTP, SSE or connector server; none for stdio and WebSocket. */
  readonly requestMs: number | undefined;
  /**
   * How long a call may go with no response and no progress notification before it aborts; `undefined`
   * when there is no idle check (`CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT` is `0`, or an IDE or SDK server).
   */
  readonly idleMs: number | undefined;
}

export interface McpTimeoutServer {
  /** `connector` is a claude.ai connector; `ide` and `sdk` are an IDE's server and an in-process SDK server. */
  readonly transport: McpTransport | 'connector' | 'ide' | 'sdk';
  /** The per-server `timeout` as written; anything but a number of at least 1000 is ignored. */
  readonly timeout?: unknown;
}

/**
 * `MCP_TIMEOUT`, `MCP_TOOL_TIMEOUT` and the server's own `timeout`. The tool timeout is the per-server
 * `timeout` (of at least 1000), else `MCP_TOOL_TIMEOUT` (below 1000 floored to 1000), else the default.
 * The per-request timer is "the greatest of three values: 60 seconds, the tool timeout that applies to
 * the server, and `MCP_TIMEOUT`", where an unset `MCP_TOOL_TIMEOUT` does not count its default.
 */
export const mcpTimeouts = (lookup: EnvLookup, server: McpTimeoutServer): McpTimeouts =>
  call('mcpTimeouts', lookup, server);

export const mcpConnectTimeoutMs = (lookup: EnvLookup): number =>
  call('mcpConnectTimeoutMs', lookup);

export interface McpDescriptionLimits {
  /** A tool description that loads upfront. */
  readonly upfront: number;
  /** A tool description loaded through tool search. */
  readonly deferred: number;
  readonly instructions: number;
}

/** `CLAUDE_CODE_MAX_MCP_DESCRIPTION_LENGTH`: "Claude Code uses that number for both limits"; anything but plain digits is ignored. */
export const mcpDescriptionLimits = (lookup: EnvLookup): McpDescriptionLimits =>
  call('mcpDescriptionLimits', lookup);

/** "Claude sees only the text before the cut"; the cut keeps a surrogate pair whole (judgment, spec/judgments.md). */
export const cutMcpDescription = (text: string, limit: number): string =>
  call('cutMcpDescription', text, limit);

/** `true`, `false`, or `undefined` for an unset, empty or unrecognised value. */
export const switchOf = (text: string | undefined): boolean | undefined => call('switchOf', text);

/**
 * Judgment (spec/judgments.md): the docs say "a safe baseline environment" and do not list it. These are
 * the variables a process needs to find programs, its home and its locale, and none holds a credential;
 * the host passes its own list when it knows better.
 */
export const MCP_ENV_BASELINE: readonly string[] = call('MCP_ENV_BASELINE');

export interface McpStdioEnvironmentInput {
  /** The environment Claude Code runs in. */
  readonly snapshot: Readonly<Record<string, string>>;
  /** The server's configured `env`, after expansion. */
  readonly configured: Readonly<Record<string, string>>;
  /** "The project root": set as `CLAUDE_PROJECT_DIR` in the server's environment only. */
  readonly projectDir?: string;
  /**
   * What the plugin exports to its server: `ResolvedMcpServer.environment`, the path variables
   * `CLAUDE_PLUGIN_ROOT` and `CLAUDE_PLUGIN_DATA` among them. Kept under the allowlist too: they are
   * the client's own per-plugin values, not the inherited shell environment it withholds (spec J3.22).
   */
  readonly exported?: Readonly<Record<string, string>>;
  readonly baseline?: readonly string[];
}

/**
 * The environment a stdio server is spawned with. "`CLAUDE_CODE_MCP_ALLOWLIST_ENV` set to `1` spawns stdio
 * MCP servers with only a safe baseline environment plus the server's configured `env`, instead of
 * inheriting your shell environment"; `CLAUDE_PROJECT_DIR` is "set in the server's environment, not in
 * Claude Code's own environment".
 */
export const mcpStdioEnvironment = (input: McpStdioEnvironmentInput): Record<string, string> =>
  call('mcpStdioEnvironment', input);

/**
 * `ENABLE_CLAUDEAI_MCP_SERVERS`: "Set to `false` to stop Claude Code from fetching claude.ai MCP servers.
 * Enabled by default for logged-in users." A value that is not an on/off spelling leaves the default.
 */
export const claudeAiConnectorsFetched = (
  lookup: EnvLookup,
  session: { readonly loggedIn: boolean },
): boolean => call('claudeAiConnectorsFetched', lookup, session);
