/**
 * The user's configuration directory (env-vars.md, `CLAUDE_CONFIG_DIR`: "Override the configuration
 * directory (default: `~/.claude`). All settings, session history, and plugins are stored under this
 * path"; claude-directory.md: "every `~/.claude` path on this page lives under that directory instead.
 * The same applies to `.claude.json`"). The one owner of that rule: every user-scope path the kit
 * builds (skills, commands, agents, output styles, settings, the plugin store, saved workflows, agent
 * memory, the `headersHelper` directory, `.claude.json`) starts here. An empty value counts as unset
 * (judgment, spec/judgments.md). Probed against Claude Code 2.1.322 as a black box: with the variable
 * set, `~/.claude/{skills,agents,commands,settings.json}` are not read and the directory's own are.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/config_directory.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/config-directory');

/** The variable, as the host's `Environment` port reads it. */
export const CONFIG_DIR_VARIABLE: string = call('CONFIG_DIR_VARIABLE');

/** `base` and a `/`-separated relative path joined the way the host writes paths (`Locations.joinPath`). */
type Join = (base: string, relativePath: string) => string;

/**
 * The configuration directory as a base directory and the folder below it: `.claude` below the home,
 * or the `CLAUDE_CONFIG_DIR` directory itself with an empty folder. A scan opens the base through the
 * host's port and reads below the folder, so no path is joined.
 */
export interface ConfigFolder {
  readonly base: string;
  /** `.claude`, or `''` when the base is the configuration directory itself. */
  readonly folder: string;
}

export const configFolderOf = (home: string, configDir: string | undefined): ConfigFolder =>
  call('configFolderOf', home, configDir);

/** The configuration directory as one path: `CLAUDE_CONFIG_DIR`, else `<home>/.claude`. */
export const configDirectoryOf = (
  home: string,
  configDir: string | undefined,
  join: Join,
): string => {
  const { base, folder } = configFolderOf(home, configDir);
  return folder === '' ? base : join(base, folder);
};

/** The global config file: `.claude.json` in `CLAUDE_CONFIG_DIR` when set, else in the home. */
export const globalConfigFileOf = (
  home: string,
  configDir: string | undefined,
  join: Join,
): string => join(configFolderOf(home, configDir).base, '.claude.json');
