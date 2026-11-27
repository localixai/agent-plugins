/**
 * A plugin id is `<name>@<origin>`: the origin is a marketplace name or one of the reserved ones
 * (`inline`, `skills-dir`, `synced`, plugins/loading.md, "Find where a plugin came from") or
 * `builtin` for a plugin inside Claude Code (settings-reference.md, `pluginConfigs`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/plugins/plugin_id.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/plugins/plugin-id');

export type PluginId = `${string}@${string}`;

export interface ParsedPluginId {
  readonly name: string;
  readonly origin: string;
}

export const parsePluginId = (text: string): ParsedPluginId | undefined =>
  call('parsePluginId', text);

export const isPluginId = (text: string): text is PluginId => call('isPluginId', text);

export type IdForms = readonly [current: PluginId, previous: PluginId];

export const agentsMdIdForms: IdForms = call('agentsMdIdForms');

export const guardIdForms: IdForms = call('guardIdForms');

export const spellingsOf = (registry: readonly IdForms[], id: string): readonly string[] =>
  call('spellingsOf', registry, id);

export const isGuardPluginId = (id: string): boolean => call('isGuardPluginId', id);
