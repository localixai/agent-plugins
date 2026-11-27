/**
 * A plugin's default settings below every settings source (plugins/components.md, "Default
 * settings").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/plugin_defaults.rs`, docs/RUST.md).
 */
import type { PluginSettings } from '../../format/plugin-settings.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/general/plugin-defaults');

/** A plugin as its defaults are merged: its name (for the debug line) and its applied settings. */
export interface PluginWithDefaults {
  readonly name: string;
  readonly settings: PluginSettings;
}

/** plugins/components.md, "Default settings": last plugin wins, below every settings source. */
export interface PluginDefaults<P extends PluginWithDefaults = PluginWithDefaults> {
  readonly settings: PluginSettings;
  /** The plugin each key of `settings` came from: the one loaded last that sets it. */
  readonly owners: { readonly [K in keyof PluginSettings]?: P };
  /** The `claude --debug` lines: `Plugin "<b>" overrides setting "<key>" set by plugin "<a>"`. */
  readonly debug: readonly string[];
}

/** The plugins' defaults merged in the order given (the load order): a later plugin's key wins. */
export const pluginDefaultsOf = <P extends PluginWithDefaults>(
  plugins: readonly P[],
): PluginDefaults<P> => call('pluginDefaultsOf', plugins);

/** A key's value after the plugin defaults: from a settings source, or from the plugin named. */
export interface SettingWithDefault<K extends keyof PluginSettings, P> {
  readonly value: NonNullable<PluginSettings[K]>;
  /** The plugin whose default it is; absent when a settings source sets the key. */
  readonly plugin?: P;
}

/**
 * The value of `agent` or `subagentStatusLine`: "plugin defaults are the lowest layer, so a user's
 * own `agent` in `~/.claude/settings.json` overrides yours". `undefined` when nothing sets it.
 */
export const valueWithPluginDefaults = <
  K extends keyof PluginSettings,
  P extends PluginWithDefaults,
>(
  view: PolicyView,
  key: K,
  defaults: PluginDefaults<P>,
): SettingWithDefault<K, P> | undefined => call('valueWithPluginDefaults', view, key, defaults);
