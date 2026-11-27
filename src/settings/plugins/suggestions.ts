/**
 * `pluginSuggestionMarketplaces`: the marketplaces whose plugins may surface as contextual install
 * suggestions (settings-reference.md, `pluginSuggestionMarketplaces`; plugins/relevance.md, "Enable
 * suggestions in managed settings"). The suggestions themselves come from each plugin's `relevance`
 * and render in the host's spinner tips and Discover tab; the kit decides which names take effect.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/plugins/suggestions.rs`, docs/RUST.md).
 */
import type { SdkMarketplaceSource } from '../../protocol/official.ts';
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/plugins/suggestions');

/** The marketplaces registered on the machine: name to the source each was registered from. */
export type RegisteredMarketplaces = Readonly<Record<string, SdkMarketplaceSource>>;

/**
 * The names of `pluginSuggestionMarketplaces` that take effect, in the order the list gives them. A name
 * takes effect only when the marketplace is registered and its registered source is also declared in the
 * same managed settings: as the entry of that name in `extraKnownMarketplaces`, or as an entry of
 * `strictKnownMarketplaces` that admits it. A marketplace registered under the name from another source is
 * ignored. The official marketplace is exempt from the source requirement: its name "can only register
 * from the official Anthropic source". The key counts from managed settings only (its **Scope** line).
 */
export const suggestionMarketplaces = (
  view: PolicyView,
  registered: RegisteredMarketplaces,
): readonly string[] => call('suggestionMarketplaces', view, registered);
