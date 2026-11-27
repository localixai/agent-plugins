import type { PluginIssue } from '../../format/issues.ts';
import type { McpCandidate } from './scope.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The MCP servers of the files Claude Code itself keeps, as candidates of their scope:
 * `~/.claude.json` and a project's `.mcp.json`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/candidates.rs`, docs/RUST.md).
 */
const call = coreModule('settings/mcp/candidates');

export interface McpCandidatesRead {
  readonly candidates: readonly McpCandidate[];
  readonly issues: readonly PluginIssue[];
}

/** The `~/.claude.json` entry of one project (`projects["<path>"]`), as it was written. */
export const claudeJsonProject = (state: unknown, projectPath: string): unknown =>
  call('claudeJsonProject', state, projectPath);

/**
 * The user and local servers of Claude Code's state file `~/.claude.json`, already parsed by the host
 * (mcp.md, "MCP installation scopes"): the top-level `mcpServers` are user scope, "available across
 * all projects"; those under `projects["<path>"]` are local scope and load "only in the project where
 * you added it". The host names the project path the way Claude Code keys it.
 */
export const claudeJsonServers = (state: unknown, projectPath: string): McpCandidatesRead =>
  call('claudeJsonServers', state, projectPath);

/**
 * The servers of a project's `.mcp.json`, already parsed (mcp.md, "Project scope"): project scope.
 * An absent file (`undefined`) holds none. They load only once approved (`projectServerApproval`); this reads, it does not decide.
 */
export const projectFileServers = (mcpJson: unknown): McpCandidatesRead =>
  call('projectFileServers', mcpJson);
