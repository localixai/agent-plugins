/**
 * The per-project lists a toggle in `/mcp` writes to `~/.claude.json` (mcp.md, "Disable a server
 * without removing it"), and which servers they leave on.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/toggles.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { McpCandidate, McpScope } from './scope.ts';

const call = coreModule('settings/mcp/toggles');

/**
 * The per-project lists a toggle in `/mcp` writes to `~/.claude.json` (mcp.md, "Disable a server
 * without removing it"). They are unrelated to `enabledMcpjsonServers` and `disabledMcpjsonServers`.
 */
export interface ProjectToggles {
  readonly disabledMcpServers: readonly string[];
  readonly enabledMcpServers: readonly string[];
}

/**
 * What a toggle applies to: the six scopes, and the built-in servers, which the docs split by their
 * default (`computer-use` "defaults to off").
 */
export type ToggleSubject = McpScope | 'builtin' | 'builtinDefaultOff';

/** The two lists of one project's entry in the state file `~/.claude.json`, already parsed. */
export const readProjectToggles = (state: unknown, projectPath: string): ProjectToggles =>
  call('readProjectToggles', state, projectPath);

/** Whether the project's lists leave the server on. A name in the list the server does not consult is ignored. */
export const serverEnabled = (
  toggles: ProjectToggles,
  subject: ToggleSubject,
  name: string,
): boolean => call('serverEnabled', toggles, subject, name);

/**
 * How a connector is written in the lists: "under its display name, for example `claude.ai Slack`"
 * (`/mcp` lists the connector Anthropic provides as `claude.ai Claude Docs`).
 */
export const connectorDisplayName = (connector: string): string =>
  call('connectorDisplayName', connector);

/** The candidates with `enabled` set from the project's lists; a disabled one stays, marked. */
export const applyToggles = (
  candidates: readonly McpCandidate[],
  toggles: ProjectToggles,
): McpCandidate[] => call('applyToggles', candidates, toggles);
