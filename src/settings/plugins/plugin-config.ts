/**
 * The answers stored for a plugin under `pluginConfigs` (settings-reference.md, `pluginConfigs`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/plugins/plugin_config.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { UserConfigValue } from '../../protocol/official.ts';
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/plugins/plugin-config');

/** What an option or a server value is: "a string, number, Boolean, or array of strings", as the SDK types it. */
export type OptionValue = UserConfigValue;

export type OptionValues = Readonly<Record<string, OptionValue>>;

export interface PluginConfig {
  /** The non-sensitive answers to the plugin's `userConfig` dialog. */
  readonly options: OptionValues;
  /** The per-server user configuration values, by MCP server name. */
  readonly mcpServers: Readonly<Record<string, OptionValues>>;
  /** What was dropped from the entry, as warnings. */
  readonly issues: readonly PluginIssue[];
}

/**
 * The answers stored for a plugin (settings-reference.md, `pluginConfigs`). Only user and managed
 * settings carry the key, so a cloned repository cannot supply them. The highest file with a usable
 * entry for the plugin supplies it whole. A built-in plugin is found under either of its ids. The
 * built-in guard's options count in managed settings only: "the same entry in a user, project, or
 * local settings file, or in a file passed with `--settings`, neither sets an option nor loosens
 * one" (plugins/mods/admin.md, "Set options on the built-in guard").
 *
 * Judgment (spec/judgments.md): entries of two files are not combined field by field, with both
 * ids of a built-in plugin in one file the current id wins, and an entry that is not an object is
 * dropped with a warning so the next file's entry applies; the docs say none of these.
 */
export const pluginConfig = (view: PolicyView, id: string): PluginConfig =>
  call('pluginConfig', view, id);
