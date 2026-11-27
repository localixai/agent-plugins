/**
 * The policy refusals a user reads (plugins/troubleshooting.md, "Policy refusals"): the line the
 * command prints, one function per entry. The host prints them; the kit only words them.
 *
 * Judgment (spec/judgments.md): the docs give each message's first line and the facts the rest
 * carries, not every word, so the second sentences below follow those facts.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/refusal_messages.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/marketplaces/refusal-messages');

export const skillsDirectoryBlockedMessage = (): string => call('skillsDirectoryBlockedMessage');

export const commandSourceDisabledMessage = (): string => call('commandSourceDisabledMessage');

export type PluginBlock =
  | { readonly kind: 'plugin'; readonly action: 'install' | 'enable' }
  | { readonly kind: 'marketplace'; readonly marketplace: string }
  | { readonly kind: 'dependency'; readonly dependency: string };

export const pluginBlockedMessage = (name: string, block: PluginBlock): string =>
  call('pluginBlockedMessage', name, block);
