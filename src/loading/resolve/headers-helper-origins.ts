/**
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/headers_helper_origins.rs`,
 * docs/RUST.md).
 */
import type { Locations } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/resolve/headers-helper-origins');

/**
 * Where an MCP server was configured, as far as its `headersHelper` is concerned (mcp.md, "Where
 * the helper runs", "Which variables a helper can read" and "Trust a folder before its
 * headersHelper runs"). The first six are the scopes of `settings/mcp` (`McpScope`, a plugin and a
 * connector included), so a ranked candidate's `scope` is one of these; the rest are the places the
 * three tables name that are not scopes: the SDK, `--mcp-config` and the five sources of an inline
 * server in an agent file.
 */
export const HEADERS_HELPER_ORIGINS: readonly [
  'plugin',
  'project',
  'local',
  'user',
  'managed',
  'connector',
  'sdk',
  'mcp-config',
  'agent-project',
  'agent-add-dir',
  'agent-user',
  'agent-managed',
  'agent-flag',
] = call('HEADERS_HELPER_ORIGINS');
export type HeadersHelperOrigin = (typeof HEADERS_HELPER_ORIGINS)[number];

/** The kinds of working directory the "Where the helper runs" table names. */
export type DirectoryKind =
  | 'plugin-root'
  | 'project-directory'
  | 'primary-directory'
  | 'config-directory';

/** What the plan reads from the host: the environment it inherits and the directories it picks from. */
export interface HelperContext {
  /** The environment the helper starts from, before any removal. */
  readonly environment: Readonly<Record<string, string>>;
  readonly locations: Pick<Locations, 'homeDir' | 'joinPath'>;
  /** The project directory a project or local server is declared in. */
  readonly projectDirectory: string;
  /** The session's primary working directory (it moves with `/cd`). */
  readonly primaryDirectory: string;
  /** The plugin's root; only a plugin-provided server has one. */
  readonly pluginRoot?: string | undefined;
}

/** "Your configuration directory, `~/.claude` unless you set `CLAUDE_CONFIG_DIR`." */
export const configDirectory = (context: HelperContext): string => call('configDirectory', context);
