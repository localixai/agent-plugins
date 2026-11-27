import type { ResolvedUserConfig, UserConfigValue } from '../loading/resolve/user-config.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput } from './result.ts';

/**
 * `claude plugin configure <plugin>` as a plan (plugins/cli-reference.md, "plugin configure"): the
 * plugin's `userConfig` options and which are set, or the values piped in on stdin validated and
 * saved. Only the full `name@marketplace` id is accepted ("`configure` doesn't accept a bare
 * `name`"). A saved value is never printed except, with `--json`, a value of an option that isn't
 * sensitive ("never the text of a sensitive one"). Pure over the loaded plugins; the host reads
 * stdin and stores what the plan names: "Non-sensitive values are saved under `pluginConfigs` ...
 * Sensitive values go to the platform's secure credential store" (manifest-reference.md, "Where
 * values are stored"). What the docs leave open is in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/configure_plan.rs`, docs/RUST.md).
 */

const call = coreModule('cli/configure-plan');

export interface ConfigureArguments {
  /** The plugin's full `name@marketplace` id. */
  readonly plugin: string;
  /** The text read from stdin; present exactly when `--values-stdin` is passed. */
  readonly valuesStdin?: string;
  readonly json?: boolean;
}

/** A plugin that loaded, with its declared options and what is saved for them. */
export interface ConfigurablePlugin {
  readonly id: string;
  readonly config: ResolvedUserConfig;
}

export interface ConfigureFacts {
  readonly loaded: readonly ConfigurablePlugin[];
  /**
   * The host could read the saved values back after `--values-stdin`; `false` leaves `unconfigured`
   * out of the `--json` object ("when they could be read back, the `unconfigured` ones"). Default `true`.
   */
  readonly readBack?: boolean;
}

/** What the host stores, split by where it goes. */
export interface SavedValues {
  /** `pluginConfigs` in the user's settings. */
  readonly settings: Readonly<Record<string, UserConfigValue>>;
  /** The platform's secure credential store. */
  readonly secrets: Readonly<Record<string, UserConfigValue>>;
}

export interface ConfigurePlan {
  readonly output: CommandOutput;
  /** Set when values are to be stored; a refused save sets nothing. */
  readonly save?: SavedValues;
}

export const SAVED_MESSAGE: string = call('SAVED_MESSAGE');
export const SAVE_FAILED: string = call('SAVE_FAILED');

/** What `claude plugin configure` prints and saves. */
export const planConfigure = (args: ConfigureArguments, facts: ConfigureFacts): ConfigurePlan =>
  call('planConfigure', args, facts);
