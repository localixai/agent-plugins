/**
 * The MCP servers scoped to a subagent (sub-agents.md, "Scope MCP servers to a subagent", "Trust
 * required for inline MCP servers"). The kit resolves; connecting and disconnecting are the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/mcp_scope.rs`, docs/RUST.md).
 */
import type { AgentSource } from '../format/agent-record.ts';
import type { AgentMcpServerSpec } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('agents/mcp-scope');

/** The restrictions the main session's MCP servers are under, as the host evaluates them. */
export interface McpPolicy {
  /**
   * Every restriction: `--strict-mcp-config`, `--bare`, managed MCP configuration and the
   * `allowedMcpServers` / `deniedMcpServers` policies.
   */
  allows(serverName: string): boolean;
  /**
   * Only the managed-settings restrictions: "`--strict-mcp-config` doesn't filter servers you pass
   * inline via `--agents` or the SDK `agents` option, since those are explicit caller input", while
   * "managed-settings restrictions apply to every subagent regardless of how it is defined".
   */
  managedAllows(serverName: string): boolean;
}

export type McpSkipReason = 'untrusted-folder' | 'policy';

/** One project's entry of `~/.claude.json`, which holds the answer to the trust dialog. */
export interface ProjectTrustEntry {
  readonly hasTrustDialogAccepted?: boolean | undefined;
}

export interface AgentFolderTrustInput {
  /** The folder whose `.claude/agents/` holds the agent file (an `--add-dir` directory included). */
  readonly agentFolder: string;
  /** The `projects` object of `~/.claude.json`, keyed by folder path as Claude Code writes it. */
  readonly projects: Readonly<Record<string, ProjectTrustEntry | undefined>>;
  /**
   * The session got trust of its own without the dialog: a `-p` or SDK session's automatic trust for
   * hooks in settings files. It "doesn't count" and is accepted only so a caller states it.
   */
  readonly automaticTrust?: boolean;
  /**
   * The folder is an `--add-dir` directory inside the trusted workspace's repository. Only one
   * "outside your trusted workspace's repository needs its own trust entry" (judgment: inside it the
   * workspace's trust stands).
   */
  readonly insideTrustedWorkspace?: boolean;
}

export interface AgentFolderTrust {
  readonly trusted: boolean;
  /** `projects["<path>"].hasTrustDialogAccepted`: the key a user sets in `~/.claude.json`. */
  readonly key: string;
}

/** The key of `~/.claude.json` that trusts a folder. */
export const trustKeyOf = (folder: string): string => call('trustKeyOf', folder);

/**
 * Whether the folder an agent file came from is trusted: its own entry says so ("trust the folder
 * the agent file came from"). A parent folder's entry does not count, and neither does the automatic
 * trust of `-p` or the SDK; the path is compared as written (judgment: the host normalizes it).
 */
export const agentFolderTrust = (input: AgentFolderTrustInput): AgentFolderTrust =>
  call('agentFolderTrust', input);

export interface AgentMcpResolution {
  /** Servers named by a string: already configured, they share the parent session's connection. */
  readonly references: readonly string[];
  /** Inline definitions, keyed by server name: connected when the subagent starts. */
  readonly inline: Readonly<Exclude<AgentMcpServerSpec, string>>;
  /** Servers Claude Code skips; it "shows a warning naming the blocked servers". */
  readonly skipped: readonly { readonly name: string; readonly reason: McpSkipReason }[];
  /**
   * The debug-log lines: "Until then, Claude Code skips every inline server in that agent file and
   * writes the exact `projects[\"<path>\"].hasTrustDialogAccepted` key". Present only when it did.
   */
  readonly debug?: readonly string[];
}

/** When a scoped server connects and goes away, for the run the agent file starts. */
export interface InlineServerLifecycle {
  readonly connectsAt: 'subagent-start' | 'session-start';
  readonly disconnectsAt: 'subagent-finish' | 'session-end';
}

/**
 * "Inline servers defined here are connected when the subagent starts ... and disconnected when it
 * finishes"; run as the main session an agent connects them "at startup alongside servers from
 * `.mcp.json` and settings files" (judgment: and they end with the session). A string reference
 * "share[s] the parent session's connection", so it has neither.
 */
export const inlineServerLifecycle = (asMainSession: boolean): InlineServerLifecycle =>
  call('inlineServerLifecycle', asMainSession);

interface McpContext {
  readonly source: AgentSource;
  readonly folderTrusted: boolean;
  /** The key to write to the debug log while an inline server waits for trust (`agentFolderTrust`). */
  readonly trustKey?: string;
  readonly policy: McpPolicy;
}

/**
 * The MCP servers an agent's `mcpServers` field gives it. A string is a reference to a configured
 * server, an entry keyed by a name is an inline definition kept as written. A plugin agent has none
 * (its record already drops the field). An inline server from a project or `--add-dir` agent file
 * is skipped until `folderTrusted` (the host's one answer: a parent folder's trust and the automatic
 * trust of `-p` do not count); every other source loads without it. A server the policy blocks is
 * skipped. Connecting and disconnecting the servers is the host's.
 */
export const resolveAgentMcpServers = (
  specs: readonly AgentMcpServerSpec[] | undefined,
  ctx: McpContext,
): AgentMcpResolution => call('resolveAgentMcpServers', specs, ctx);
