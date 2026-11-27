import type { MarketplaceSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * `Claude Code refuses the marketplace name` (errors.md): Claude Code "checks the name every time it
 * reads the marketplace's catalog", so a marketplace registered under an imitating name before the
 * check blocked it stops loading, and so do its plugins. The host calls this on every load beside
 * `ignoredReason` and `untrustedSource`. The advice after the quoted prefix is the kit's wording
 * (spec/judgments.md); the steps for a marketplace declared inline in settings are the docs'
 * (v2.1.292 or later).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/refused_name.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/refused-name');

export interface RefusedMarketplaceName {
  /** The marketplace's own error. */
  readonly marketplace: string;
  /** What `claude plugin list` and the `/plugin` Errors tab show for each of its plugins. */
  readonly plugin: string;
  /** For a marketplace a settings file declares inline (`"source": "settings"`): the steps that keep it. */
  readonly steps?: readonly string[];
}

/** The refusal of a registered marketplace whose name imitates an official one, or `undefined` when its name loads. */
export const refusedMarketplaceName = (
  name: string,
  source: MarketplaceSource,
): RefusedMarketplaceName | undefined => call('refusedMarketplaceName', name, source);
