/**
 * Reading a marketplace's catalog once the host has it (plugins/marketplace-reference.md, "Fields by
 * type"): the text of a `url` download or a `file` source, or a `directory` source, "the directory
 * that contains `.claude-plugin/marketplace.json`". Both are `readMarketplace` over the parsed file;
 * invalid JSON or a missing file keeps the marketplace from loading, never a throw.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/marketplace_catalog.rs`, docs/RUST.md).
 */
import type { MarketplaceContext, MarketplaceResult } from '../format/marketplace/types.ts';
import type { PluginFiles } from '../ports/files.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/marketplace-catalog');

export const MARKETPLACE_FILE: '.claude-plugin/marketplace.json' = call('MARKETPLACE_FILE');

/** The marketplace a `marketplace.json` text holds. */
export const readMarketplaceText = (text: string, context: MarketplaceContext): MarketplaceResult =>
  call('readMarketplaceText', text, context);

/** The marketplace of a directory, through `files` rooted at it. */
export const readMarketplaceDirectory = (
  files: PluginFiles,
  context: MarketplaceContext,
  signal?: AbortSignal,
): Promise<MarketplaceResult> => call.async('readMarketplaceDirectory', files, context, signal);
