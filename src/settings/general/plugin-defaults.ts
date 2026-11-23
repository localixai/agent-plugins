import { PLUGIN_AGENT_DEPRECATED, type PluginSettings } from '../../format/plugin-settings.ts';
import type { PolicyView } from '../view.ts';

/** A plugin as its defaults are merged: its name (for the debug line) and its applied settings. */
export interface PluginWithDefaults {
  readonly name: string;
  readonly settings: PluginSettings;
}

/** A startup warning: an enabled plugin sets the deprecated `agent` default (spec §12.6 item 2). */
export interface PluginDefaultWarning<P extends PluginWithDefaults = PluginWithDefaults> {
  readonly plugin: P;
  readonly message: string;
}

/** Spec §12.6, a deprecated feature: last plugin wins, below every settings source. */
export interface PluginDefaults<P extends PluginWithDefaults = PluginWithDefaults> {
  readonly settings: PluginSettings;
  /** The plugin each key of `settings` came from: the one loaded last that sets it. */
  readonly owners: { readonly [K in keyof PluginSettings]?: P };
  /** The `claude --debug` lines: `Plugin "<b>" overrides setting "<key>" set by plugin "<a>"`. */
  readonly debug: readonly string[];
  /**
   * One deprecation warning per plugin that sets `agent`, in load order, whether or not its value
   * applies; the text is the validator's (Judgment J12.14).
   */
  readonly warnings: readonly PluginDefaultWarning<P>[];
}

const KEYS = ['agent'] as const satisfies readonly (keyof PluginSettings)[];

/** The plugins' defaults merged in the order given (the load order): a later plugin's key wins. */
export function pluginDefaultsOf<P extends PluginWithDefaults>(
  plugins: readonly P[],
): PluginDefaults<P> {
  const owners: { [K in keyof PluginSettings]?: P } = {};
  const settings: PluginSettings = {};
  const debug: string[] = [];
  const apply = <K extends keyof PluginSettings>(plugin: P, key: K) => {
    const value = plugin.settings[key];
    if (value === undefined) return;
    const previous = owners[key];
    if (previous !== undefined)
      debug.push(
        `Plugin "${plugin.name}" overrides setting "${key}" set by plugin "${previous.name}"`,
      );
    settings[key] = value;
    owners[key] = plugin;
  };
  for (const plugin of plugins) for (const key of KEYS) apply(plugin, key);
  const warnings = plugins
    .filter((plugin) => plugin.settings.agent !== undefined)
    .map((plugin) => ({ plugin, message: PLUGIN_AGENT_DEPRECATED }));
  return { settings, owners, debug, warnings };
}

/** A key's value after the plugin defaults: from a settings source, or from the plugin named. */
export interface SettingWithDefault<K extends keyof PluginSettings, P> {
  readonly value: NonNullable<PluginSettings[K]>;
  /** The plugin whose default it is; absent when a settings source sets the key. */
  readonly plugin?: P;
}

/** What the settings sources themselves set of the key. */
const ownSettings = (view: PolicyView): PluginSettings => {
  const agent = view.get('agent');
  return agent === undefined ? {} : { agent };
};

/**
 * The value of `agent`: "Plugin defaults are the lowest settings layer: an `agent` at any settings
 * level, the user's own in `~/.claude/settings.json` for one, replaces the plugin's" (§12.6 item 4).
 * `undefined` when nothing sets it.
 */
export function valueWithPluginDefaults<
  K extends keyof PluginSettings,
  P extends PluginWithDefaults,
>(view: PolicyView, key: K, defaults: PluginDefaults<P>): SettingWithDefault<K, P> | undefined {
  const set = ownSettings(view)[key];
  if (set !== undefined) return { value: set };
  const value = defaults.settings[key];
  const plugin = defaults.owners[key];
  return value === undefined || plugin === undefined ? undefined : { value, plugin };
}
