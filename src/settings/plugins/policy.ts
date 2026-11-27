import type { PolicyView } from '../view.ts';
import type { EnabledDecision } from './enabled-plugins.ts';
import type { ModOrderContext, ModOrderResult } from './mod-order.ts';
import type { PluginConfig } from './plugin-config.ts';
import type { RegisteredMarketplaces } from './suggestions.ts';
import type { SyncDecision } from './sync-plugins.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/plugins/policy');

/**
 * The plugin keys of the settings as one object over a view (docs/API.md, "Settings").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/plugins/policy.rs`, docs/RUST.md).
 */
export interface PluginSettingsPolicy {
  enabled(id: string, manifestDefaultEnabled?: boolean): EnabledDecision;
  forceEnabledByManaged(id: string): boolean;
  config(id: string): PluginConfig;
  modOrder(orgMods: ReadonlySet<string>): ModOrderResult;
  syncClaudeAiPlugins(): SyncDecision;
  trustWarning(standard?: string): string;
  suggestionMarketplaces(registered: RegisteredMarketplaces): readonly string[];
}

export const createPluginSettingsPolicy = (
  view: PolicyView,
  context: ModOrderContext,
): PluginSettingsPolicy =>
  call('createPluginSettingsPolicy', view, {
    hasManagedSettings: context.hasManagedSettings,
    account: context.account,
  });
