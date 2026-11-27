import type { PluginIssue } from '../format/issues.ts';
import type { MarketplaceSource } from '../format/marketplace/types.ts';
import type { MarketplaceFetchPlan } from './types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Planning how a marketplace source is fetched (plugins/marketplace-reference.md, "Marketplace
 * sources" and "Fields by type"; plugins/host-marketplace.md, "Stay within the download limits for
 * hosted files"): pure, a source in, the plan or the issue that refuses it out. Nothing is read
 * here; the host runs the plan. Issue paths are relative to the source object.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/marketplace_fetch_plan.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/marketplace-fetch-plan');

/** How the marketplace file is fetched, or why it cannot be. */
export const planMarketplaceFetch = (
  source: MarketplaceSource,
): MarketplaceFetchPlan | PluginIssue => call('planMarketplaceFetch', source);
