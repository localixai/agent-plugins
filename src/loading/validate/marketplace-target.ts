import type { ValidationIssue } from '../../format/validate/finding.ts';
import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * Validating a marketplace folder: `.claude-plugin/marketplace.json`, then the `plugin.json` of
 * each entry whose source is a folder inside the marketplace.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/marketplace_target.rs`,
 * docs/RUST.md).
 */
const call = coreModule('loading/validate/marketplace-target');

export interface MarketplaceFolder {
  readonly issues: ValidationIssue[];
  /** The marketplace is one of Anthropic's own. */
  readonly official: boolean;
}

export const validateMarketplaceFolder = (
  files: PluginFiles,
  signal?: AbortSignal,
): Promise<MarketplaceFolder> => call.async('validateMarketplaceFolder', files, signal);
