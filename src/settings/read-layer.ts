/**
 * Turns the JSON of one settings source into a layer every resolver can trust: aliases rewritten to
 * their canonical key, a key the source may not carry dropped with a warning, an unreadable value
 * repaired as the docs say (managed-settings.md, "Find entries Claude Code dropped"). It never
 * throws; every drop and repair is a `PluginIssue`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/read_layer.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PolicyScope, PolicyLayer } from './layers.ts';

const call = coreModule('settings/read-layer');

/** An absent source (`undefined`) is an empty layer; a document that is no JSON object is skipped. */
export const readSettingsLayer = (
  scope: PolicyScope,
  raw: unknown,
  source?: string,
): { layer: PolicyLayer; issues: readonly PluginIssue[] } =>
  call('readSettingsLayer', scope, raw, source);
