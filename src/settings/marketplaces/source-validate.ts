/**
 * What an `extraKnownMarketplaces` entry's `source` must be to register a marketplace
 * (settings-reference.md, `extraKnownMarketplaces` "Marketplace source types"; plugins/marketplace-reference.md
 * is the marketplace stream's). One registry over every source kind, so a new kind fails to compile.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/source_validate.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { SdkMarketplaceSource } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/marketplaces/source-validate');

/**
 * Why the entry named `key` cannot register a marketplace; empty when it can. A kind the docs and the
 * SDK do not list is not a marketplace either: the repair of the key only checks for a `source` object.
 */
export const registrationProblems = (
  key: string,
  source: SdkMarketplaceSource,
): readonly PluginIssue[] => call('registrationProblems', key, source);
