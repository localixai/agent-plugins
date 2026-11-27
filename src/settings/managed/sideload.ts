/**
 * `disableSideloadFlags` (settings-reference.md, `disableSideloadFlags`; plugins/troubleshooting.md,
 * "`--plugin-dir is disabled by your organization's managed settings (disableSideloadFlags)`";
 * agent-sdk/typescript.md, `addPlugins`): which command-line sideloads managed settings reject and
 * what a cloud session does instead. The kit reads no command line; the host says what was given.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/sideload.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/managed/sideload');

/** One server of a `--mcp-config`. `claudeTagSlack` marks a Claude Tag session's Slack tools. */
export interface ConfiguredServer {
  readonly name: string;
  readonly type?: string;
  readonly claudeTagSlack?: boolean;
}

/** What the command line and environment of a session carry. */
export interface SideloadFacts {
  readonly pluginDirs: readonly string[];
  readonly pluginUrls: readonly string[];
  /** `--agents` was given. */
  readonly agents: boolean;
  /** The servers of `--mcp-config`; empty when the flag was not given. */
  readonly mcpServers: readonly ConfiguredServer[];
  /** The directories `CLAUDE_CODE_PLUGIN_DIRS` names (see `parsePluginDirs`). */
  readonly pluginDirsVariable: readonly string[];
  /** The session is a cloud session. */
  readonly cloud: boolean;
  /** The Agent SDK `plugins` option named plugin folders (plugins/org.md, control matrix). */
  readonly sdkPlugins?: boolean;
}

export interface SideloadVerdict {
  /**
   * Startup exits with these lines: one per rejected flag, the Agent SDK `plugins` option or the
   * variable, then `SIDELOAD_CLOSING`; empty when the session starts.
   */
  readonly refusals: readonly string[];
  /** The `--mcp-config` servers a cloud session starts without. */
  readonly droppedServers: readonly string[];
}

/** The line after the rejected flags (plugins/troubleshooting.md, "`--plugin-dir is disabled …`"). */
export const SIDELOAD_CLOSING: string = call('SIDELOAD_CLOSING');

/** What the session does with its flags and the plugin-folder variable under managed `disableSideloadFlags`. */
export const sideloadVerdict = (view: PolicyView, facts: SideloadFacts): SideloadVerdict =>
  call('sideloadVerdict', view, facts);

/**
 * The Agent SDK's `addPlugins` option under `disableSideloadFlags`: "that call fails with an error.
 * The session keeps running with the plugins it already had." `undefined` when the call proceeds.
 */
export const addPluginsRefusal = (view: PolicyView): string | undefined =>
  call('addPluginsRefusal', view);

/** The facts of a mid-session MCP update. */
export interface McpUpdateFacts {
  /** The session is a cloud session. */
  readonly cloud: boolean;
}

export interface McpUpdateVerdict {
  /** The servers of the update the session applies. */
  readonly applied: readonly ConfiguredServer[];
  /** The servers it ignores. */
  readonly ignored: readonly ConfiguredServer[];
}

/**
 * Server-delivered mid-session MCP updates, "the path behind cloud session configuration and SDK
 * `setMcpServers()` calls that reach those sessions" (settings-reference.md, `disableSideloadFlags`):
 * under managed `disableSideloadFlags` a cloud session ignores them, except "in-process `type: "sdk"`
 * entries and a Claude Tag session's Slack tools". Any other session applies the update.
 */
export const midSessionMcpUpdates = (
  view: PolicyView,
  facts: McpUpdateFacts,
  servers: readonly ConfiguredServer[],
): McpUpdateVerdict => call('midSessionMcpUpdates', view, { cloud: facts.cloud }, servers);
