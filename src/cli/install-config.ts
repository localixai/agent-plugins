import type { UserConfigValue } from '../loading/resolve/user-config.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ConfigProblem, DeclaredOptions } from './user-config-input.ts';
import type { SavedValues } from './configure-plan.ts';

/**
 * `plugin install --config <key=value>` (plugins/cli-reference.md, "plugin install"; plugins/
 * troubleshooting.md, "The `userConfig` dialog never appears"; plugins/components.md, "Include a
 * packaged MCPB server"): each flag sets "a `userConfig` option the plugin's manifest declares";
 * "A key written `<server>.<key>` sets a setting that a bundled MCP server declares in its own
 * `user_config` instead". The install "never prompts": a key the manifest doesn't declare still
 * installs the plugin and prints `⚠ Installed, but --config not applied: ...`; options left unset
 * print `N userConfig options not yet set — run /plugin configure <id> in Claude Code, or pass
 * --config KEY=VALUE.` with `(M required)` after `not yet set` when one is required. Values are
 * checked like `plugin configure`'s (`checkGiven`). A value that fails the check refuses the
 * install (judgment): the docs name the refusal for `configure` only. A key given twice keeps its
 * last value.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/install_config.rs`, docs/RUST.md).
 */

const call = coreModule('cli/install-config');

/** The `userConfig` of the plugin and of each bundled MCP server (by server name). */
export interface InstallOptions {
  readonly declared: DeclaredOptions;
  readonly bundledServers?: Readonly<Record<string, DeclaredOptions>>;
  /**
   * Bundled MCP servers whose bundle the manifest references by URL: "isn't read at install time, so
   * its keys aren't listed". Their keys are not known keys, and the message says to configure them in `/plugin`.
   */
  readonly unreadServers?: readonly string[];
}

export interface ConfigOutcome {
  /** Failing values: the install is refused. */
  readonly refused: readonly ConfigProblem[];
  /** Keys that name no option: the install goes ahead and says they were not applied. */
  readonly notApplied: readonly ConfigProblem[];
  /** What to store: the plugin's own options, and each bundled server's by server name. */
  readonly plugin: SavedValues;
  readonly servers: Readonly<Record<string, Readonly<Record<string, UserConfigValue>>>>;
  /** The plugin's own options still unset after the install. */
  readonly unset: readonly string[];
  readonly unsetRequired: number;
}

/** What the `--config` values do to an install. */
export const applyConfig = (
  given: Readonly<Record<string, string>>,
  options: InstallOptions,
): ConfigOutcome => call('applyConfig', given, options);

/** `N userConfig options not yet set (M required) — run /plugin configure <id> in Claude Code, or pass --config KEY=VALUE.` */
export const notYetSetLine = (id: string, outcome: ConfigOutcome): string | undefined =>
  call('notYetSetLine', id, outcome);
