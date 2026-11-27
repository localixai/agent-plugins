import { coreModule } from '../wasm/bridge.ts';

/**
 * The plugins that are not installed: the origins `inline` and `skills-dir` (plugins/loading.md,
 * "Find where a plugin came from"), how a session-only plugin is listed and which copy of a name
 * wins (plugins/cli-reference.md, "Flags that load a plugin for one session"), and how to stop
 * loading a skills-directory plugin (plugins/create.md, "Stop loading the plugin"). Pure.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/session_origins.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/session-origins');

/** `<name>@inline`: "a plugin that `--plugin-dir`, `--plugin-url`, `CLAUDE_CODE_PLUGIN_DIRS` or the Agent SDK `plugins` option loaded". */
export const inlinePluginId = (name: string): string => call('inlinePluginId', name);

/** `<name>@skills-dir`: "`skills-dir` stands where a marketplace name would". */
export const skillsDirPluginId = (name: string): string => call('skillsDirPluginId', name);

/**
 * How `claude plugin list` shows a session-only plugin: "only when the same flag precedes the
 * subcommand, as in `claude --plugin-dir ./my-plugin plugin list`", as `<name>@inline` under a
 * heading that begins `Session-only plugins`, and `--json` reports its `scope` as `session`.
 */
export interface SessionOnlyRow {
  readonly id: string;
  readonly heading: string;
  readonly scope: 'session';
}

export const sessionOnlyRow = (
  name: string,
  flagPrecedesSubcommand: boolean,
): SessionOnlyRow | undefined => call('sessionOnlyRow', name, flagPrecedesSubcommand);

export interface NameClash {
  /** The session-only copy was disabled with `claude plugin disable <name>@inline`. */
  readonly sessionOnlyDisabled: boolean;
  /** Managed settings lock that plugin name. */
  readonly managedLocksName: boolean;
}

/**
 * "When a session-only plugin shares a name with an installed plugin, Claude Code loads the
 * session-only copy for that session and skips the installed one. The installed copy loads instead
 * if you disabled the session-only copy with `claude plugin disable <name>@inline`, or if managed
 * settings lock that plugin name."
 */
export const copyThatLoads = (clash: NameClash): 'session-only' | 'installed' =>
  call('copyThatLoads', clash);

/**
 * The two ways to stop loading a scaffolded plugin: "delete its directory, or run `claude plugin
 * disable my-tool@skills-dir` in your shell".
 */
export const stopLoadingSkillsDirPlugin = (
  name: string,
): {
  readonly deleteDirectory: true;
  readonly disableCommand: string;
} => call('stopLoadingSkillsDirPlugin', name);
