/**
 * The `env` block of the settings files for a host that knows its folders' trust per file (env-vars.md,
 * "Set variables in settings files" and "Precedence"; settings-reference.md, `env`). The same rules as
 * `resolveEnv`, in the shape a host applies directly: the values to write over its environment.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/env/settings_env.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyLayer } from '../layers.ts';
import type { NotAppliedVariable, Variables } from './resolve-env.ts';

const call = coreModule('settings/env/settings-env');

/** What `resolveSettingsEnv` asks the host about its machine. */
export interface SettingsEnvContext {
  /** Whether the user trusted the workspace (settings-reference.md, "When Claude Code applies `env` values"). */
  readonly isFolderTrusted: (scope: 'project' | 'local') => boolean;
  /** The environment Claude Code was launched with: an off value of a project file never replaces one set here. */
  readonly environment?: Variables;
  /**
   * Claude Desktop or a self-hosted runner built the launch environment: "Claude Code ignores an `env`
   * value from any settings file for a variable the launch environment already sets."
   */
  readonly launchOwnsEnvironment?: boolean;
  /** The host's variable names are case-insensitive. */
  readonly windows?: boolean;
}

export interface SettingsEnv {
  /** The variables to put in the environment, replacing what the shell exported. */
  readonly values: Readonly<Record<string, string>>;
  /** The variables of a file that do not reach the environment, and why. Never a value. */
  readonly ignored: readonly NotAppliedVariable[];
}

/** The variables of the layers' `env` blocks that reach the environment. */
export const resolveSettingsEnv = (
  layers: readonly PolicyLayer[],
  context: SettingsEnvContext,
): SettingsEnv => call('resolveSettingsEnv', layers, context);

/**
 * The environment after the settings' variables are written into it: "Claude Code writes each `env`
 * entry into the process environment, replacing the value inherited from the shell". Nothing is ever
 * removed: "In a settings file you can set a variable but you can't remove one". On Windows a name
 * replaces the inherited one whatever its case.
 */
export const applySettingsEnv = (
  inherited: Readonly<Record<string, string>>,
  settings: Readonly<Record<string, string>>,
  windows: boolean,
): Record<string, string> => call('applySettingsEnv', inherited, settings, windows);
