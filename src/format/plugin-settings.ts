/**
 * A plugin's default settings (plugins/components.md, "Default settings"; plugins/manifest-reference.md,
 * "`settings`"): the `settings.json` at the plugin root or the inline `settings` manifest key. "Two
 * keys take effect, `agent` and `subagentStatusLine`, and every other key is dropped." The types are
 * the official `Settings` ones; applying the values (the lowest settings layer) is the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/plugin_settings.rs`, docs/RUST.md).
 */
import type { Settings } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

const call = coreModule('format/plugin-settings');

export type PluginSettings = Pick<Settings, 'agent' | 'subagentStatusLine'>;

export interface PluginSettingsRead {
  /** Only the keys that take effect and hold a valid value. */
  readonly settings: PluginSettings;
  readonly issues: readonly PluginIssue[];
}

/** True when a read holds at least one applied key: `settings.json` then wins over the manifest. */
export const hasAppliedSettings = (read: PluginSettingsRead): boolean =>
  call('hasAppliedSettings', read);

/**
 * Reads one settings object. Judgment (spec/judgments.md): a dropped key and an invalid value for an
 * applied key are warnings (the docs say "dropped" and name no message), and a value that is not an
 * object is a warning naming `origin`: a plugin's defaults never fail the plugin (spec §12.6 item 1,
 * J12.10).
 */
export const readPluginSettings = (raw: unknown, origin: string): PluginSettingsRead =>
  call('readPluginSettings', raw, origin);
