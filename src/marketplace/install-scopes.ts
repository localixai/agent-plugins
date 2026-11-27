/**
 * The settings scopes a plugin can be enabled or installed in (plugins/host-marketplace.md, "Migrate
 * users with a renames map" and "Uninstall removed plugins from users' machines": "the user,
 * project, and local settings scopes"; managed settings are the one the kit and Claude Code cannot
 * write). A scope is a name only; reading the settings of each is the settings stream's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/install_scopes.rs`, docs/RUST.md).
 */
import type { ResolvedSettingSource } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/install-scopes');

export const INSTALL_SCOPES: readonly InstallScope[] = call('INSTALL_SCOPES');

/** The SDK's setting sources but `flag` (the `--settings` tier), which the docs do not name as a scope a plugin is installed in. */
export type InstallScope = Exclude<ResolvedSettingSource, 'flag'>;

/** Whether the user can have a plugin changed in the scope: every scope but `managed`. */
export const isWritableScope = (scope: InstallScope): boolean => call('isWritableScope', scope);

/** Who a scope reaches, and in which projects (plugins/install.md, "Choose an install scope"; plugins/overview.md, "Understand install scopes"). */
export interface ScopeReach {
  /** The settings file that records the plugin as enabled, as the docs write it. */
  readonly file: string;
  readonly audience: 'you' | 'everyone-in-repository';
  readonly reach: 'every-project' | 'this-repository';
  /** The file is checked into the repository. */
  readonly committed: boolean;
  /** Each collaborator still installs the plugin on their own machine. */
  readonly collaboratorsInstall: boolean;
}

/** The scopes a user chooses between in a command or the `/plugin` panel: the three settings files. */
export type ChosenScope = 'user' | 'project' | 'local';

export const scopeReach = (scope: ChosenScope): ScopeReach => call('scopeReach', scope);

/**
 * The scopes from the one that wins to the one that loses, the one precedence table: "the local
 * setting overrides the project setting, and the project setting overrides the user setting", and
 * managed settings override every other file (settings-reference.md, `enabledPlugins`).
 */
export const SCOPES_BY_PRECEDENCE: readonly InstallScope[] = call('SCOPES_BY_PRECEDENCE');

/** The highest-precedence scope among `scopes` that has a settings file; `managed` has none the user writes. */
export const mostSpecificScope = (scopes: readonly InstallScope[]): ChosenScope | undefined =>
  call('mostSpecificScope', scopes);

/** What a collaborator runs once after the project's `.claude/settings.json` enables the plugin. */
export const collaboratorInstallCommand = (pluginId: string): string =>
  call('collaboratorInstallCommand', pluginId);

/** The shell commands that take `--scope` and default when it is omitted (plugins/install.md, "Manage plugins from your shell"). */
export type ScopedCommand = 'install' | 'uninstall' | 'enable' | 'disable';

/**
 * The scope a command acts on when `--scope` is omitted: `install` and `uninstall` use user scope;
 * `enable` and `disable` use "the most specific scope whose settings already list the plugin"
 * (`listedIn`), none when no settings file lists it.
 */
export const defaultScope = (
  command: ScopedCommand,
  listedIn: readonly InstallScope[],
): ChosenScope | undefined => call('defaultScope', command, listedIn);
