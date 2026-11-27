import type { UserConfigOption } from '../../format/index.ts';
import type { UserConfigValue } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

export type { UserConfigValue };

const call = coreModule('loading/resolve/user-config');

/**
 * What the user (or the host) saved for a plugin's `userConfig` options
 * (plugins/manifest-reference.md, "User configuration"). The kit never stores or prompts: the
 * host reads `pluginConfigs` and its secure store and hands the values over as plain data.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/user_config.rs`,
 * docs/RUST.md).
 */
export interface ResolvedUserConfig {
  /** The manifest's `userConfig`. */
  readonly declared: Readonly<Record<string, UserConfigOption>>;
  /**
   * The saved values by key, sensitive ones included. A key nobody declared is ignored. A bundled
   * server's settings are here too, under `<server>.<key>` (`bundleSettingKey`, the `--config` form):
   * only that server reads them (`bundleScope`).
   */
  readonly values: Readonly<Record<string, UserConfigValue>>;
}

/** The process variable a hook reads an option from: `api_token` is `CLAUDE_PLUGIN_OPTION_API_TOKEN`. */
export const optionEnvName = (key: string): string => call('optionEnvName', key);

/**
 * The value `${user_config.KEY}` stands for: the saved one, else the declared `default` ("Value
 * used when the user provides nothing"), else `undefined`. Judgment: a key the manifest does
 * not declare has no value, whatever the host supplied.
 */
export const userConfigValue = (
  config: ResolvedUserConfig,
  key: string,
): UserConfigValue | undefined => call('userConfigValue', config, key);

/**
 * A value as text. The docs show strings only; judgment: a number or boolean is written as
 * `String(value)`, a list joined by commas (one argument, no spaces to split on).
 */
export const userConfigText = (value: UserConfigValue): string => call('userConfigText', value);

/**
 * True when a template references `${user_config.*}`. It reads the template, never a value, so it
 * fires when nothing is configured yet (errors.md, "plugin-command-references-user-config"): a
 * shell field is refused for what it says, not for what is saved.
 */
export const referencesUserConfig = (template: string): boolean =>
  call('referencesUserConfig', template);

/**
 * The variables a hook process receives for the options (plugins/manifest-reference.md,
 * "Reference a saved value"): one per option, sensitive ones included. Judgment: an option
 * with neither a saved value nor a default has nothing to export and is left out.
 */
export const optionEnvironment = (config: ResolvedUserConfig): Record<string, string> =>
  call('optionEnvironment', config);
