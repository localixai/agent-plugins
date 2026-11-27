/**
 * The approvals of a project's `.mcp.json` servers (mcp.md, "Project server approvals and workspace
 * trust"), and whether the session can show the approval prompt.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/approvals.rs`, docs/RUST.md).
 */
import type { PolicyLayer } from '../layers.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/approvals');

/**
 * What the host knows about the folder (mcp.md, "Project server approvals and workspace trust"). The
 * kit runs no git and reads no trust dialog result: the host answers these.
 */
export interface McpTrust {
  /** The user accepted the workspace trust dialog for this folder. */
  readonly folderTrusted: boolean;
  /** The folder is "your own configuration home: your home directory, or a directory whose `.claude` you've set as `CLAUDE_CONFIG_DIR`". */
  readonly configHome?: boolean;
  /** git tracks `.claude/settings.local.json`; the docs exempt only an "untracked" one (judgment). */
  readonly localTracked?: boolean;
}

/**
 * Two of the statuses `claude mcp list` and `claude mcp get` print without connecting (mcp.md,
 * "Server status"): `pending approval` shows in both commands, `rejected` only in `claude mcp get`.
 * `✘ Blocked by enterprise managed policy` is a third such status; a policy decision is not an
 * approval decision, so it is out of scope here (spec/judgments.md).
 */
export const PROJECT_SERVER_STATUSES: readonly ['approved', 'pending approval', 'rejected'] =
  call('PROJECT_SERVER_STATUSES');
export type ProjectServerStatus = (typeof PROJECT_SERVER_STATUSES)[number];

/** What decided a server's status; absent while it is pending. */
export type ApprovalVia =
  | 'disabledMcpjsonServers'
  | 'enableAllProjectMcpServers'
  | 'enabledMcpjsonServers'
  | 'session';

export interface ProjectServerApproval {
  readonly name: string;
  readonly status: ProjectServerStatus;
  readonly via?: ApprovalVia;
}

/** Whether the session can show the approval prompt (`canPromptForProjectServers`). */
export interface ApprovalSession {
  readonly prompts: boolean;
}

/**
 * The status of each server of a project's `.mcp.json` (`servers` are its names, in the order to
 * report). A server is pending until `enableAllProjectMcpServers` or `enabledMcpjsonServers` approves
 * it; a `disabledMcpjsonServers` entry in any file, honoured or not, rejects it and takes precedence.
 * A session that cannot prompt (`claude -p`, Agent SDK, cloud) loads a pending server without asking.
 */
export const projectServerApproval = (
  servers: readonly string[],
  layers: readonly PolicyLayer[],
  trust: McpTrust,
  session: ApprovalSession = { prompts: true },
): readonly ProjectServerApproval[] =>
  call('projectServerApproval', servers, layers, trust, session);

export interface PromptSession {
  /** An interactive session; `claude -p`, Agent SDK and cloud sessions "can't show that prompt". */
  readonly interactive: boolean;
  /** The session runs in `bypassPermissions` mode. */
  readonly bypassPermissions: boolean;
}

/**
 * Whether the session shows the approval prompt (mcp.md, "Project scope"): interactive sessions do,
 * except one in `bypassPermissions` mode with `skipDangerousModePermissionPrompt` set in user or
 * managed settings.
 */
export const canPromptForProjectServers = (
  session: PromptSession,
  layers: readonly PolicyLayer[],
): boolean => call('canPromptForProjectServers', session, layers);
