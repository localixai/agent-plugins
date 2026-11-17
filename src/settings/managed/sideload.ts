import type { PolicyView } from '../view.ts';

/**
 * `disableSideloadFlags` (settings-reference.md, `disableSideloadFlags`; plugins/troubleshooting.md,
 * "`--plugin-dir is disabled by your organization's managed settings (disableSideloadFlags)`";
 * agent-sdk/typescript.md, `addPlugins`): which command-line sideloads managed settings reject and
 * what a cloud session does instead. The kit reads no command line; the host says what was given.
 */
export const SIDELOAD_FLAGS = ['--plugin-dir', '--plugin-url', '--agents', '--mcp-config'] as const;
export type SideloadFlag = (typeof SIDELOAD_FLAGS)[number];

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

/** "Claude Code still accepts a `--mcp-config` whose servers are all in-process `type: "sdk"` entries." */
const IN_PROCESS_TYPES: ReadonlySet<string | undefined> = new Set(['sdk']);
const isInProcess = (server: ConfiguredServer): boolean => IN_PROCESS_TYPES.has(server.type);

/** What each flag has to carry to be rejected: `--mcp-config` only when a server is not in process. */
const REJECTED_WHEN: { readonly [F in SideloadFlag]: (facts: SideloadFacts) => boolean } = {
  '--plugin-dir': (facts) => facts.pluginDirs.length > 0,
  '--plugin-url': (facts) => facts.pluginUrls.length > 0,
  '--agents': (facts) => facts.agents,
  '--mcp-config': (facts) =>
    !facts.cloud && facts.mcpServers.some((server) => !isInProcess(server)),
};

const refusalOf = (flag: string): string =>
  `${flag} is disabled by your organization's managed settings (disableSideloadFlags)`;

/** The line after the rejected flags (plugins/troubleshooting.md, "`--plugin-dir is disabled …`"). */
export const SIDELOAD_CLOSING =
  'Plugins, custom agents, and MCP servers can only be loaded from sources your administrator has approved.';

/** The line for the Agent SDK `plugins` option, in the flags' form: its name is the kit's (spec/judgments.md). */
const SDK_PLUGINS = 'The Agent SDK plugins option';

/** The rejected flags' lines, then the SDK option's and the variable's, then the closing line. */
function refusalLines(facts: SideloadFacts): readonly string[] {
  const lines = [
    ...SIDELOAD_FLAGS.filter((flag) => REJECTED_WHEN[flag](facts)).map(refusalOf),
    ...(facts.sdkPlugins === true ? [refusalOf(SDK_PLUGINS)] : []),
    ...(facts.pluginDirsVariable.length > 0
      ? [`${refusalOf('CLAUDE_CODE_PLUGIN_DIRS')}: unset the variable`]
      : []),
  ];
  return lines.length === 0 ? [] : [...lines, SIDELOAD_CLOSING];
}

/** In a cloud session a server survives only as "in-process `type: "sdk"` entries and a Claude Tag session's Slack tools". */
const survivesInCloud = (server: ConfiguredServer): boolean =>
  isInProcess(server) || server.claudeTagSlack === true;

/** What the session does with its flags and the plugin-folder variable under managed `disableSideloadFlags`. */
export function sideloadVerdict(view: PolicyView, facts: SideloadFacts): SideloadVerdict {
  if (view.get('disableSideloadFlags') !== true) return { refusals: [], droppedServers: [] };
  const dropped = facts.cloud
    ? facts.mcpServers.filter((server) => !survivesInCloud(server)).map((server) => server.name)
    : [];
  return { refusals: refusalLines(facts), droppedServers: dropped };
}

/**
 * The Agent SDK's `addPlugins` option under `disableSideloadFlags`: "that call fails with an error.
 * The session keeps running with the plugins it already had." `undefined` when the call proceeds.
 */
export function addPluginsRefusal(view: PolicyView): string | undefined {
  return view.get('disableSideloadFlags') === true
    ? 'reload_plugins add_plugins is disabled by managed settings (disableSideloadFlags)'
    : undefined;
}

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
export function midSessionMcpUpdates(
  view: PolicyView,
  facts: McpUpdateFacts,
  servers: readonly ConfiguredServer[],
): McpUpdateVerdict {
  if (!facts.cloud || view.get('disableSideloadFlags') !== true) {
    return { applied: servers, ignored: [] };
  }
  return {
    applied: servers.filter(survivesInCloud),
    ignored: servers.filter((server) => !survivesInCloud(server)),
  };
}
