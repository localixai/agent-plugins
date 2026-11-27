/**
 * Marketplace names (plugins/marketplace-reference.md, "Reserved names" and "Validation messages";
 * errors.md, "Plugin errors"; plugins/troubleshooting.md, "Cannot add marketplace"). The grammar
 * and the control-character rule are `ids.ts`; this file holds the lists the docs print and the
 * checks they describe.
 *
 * The docs split the rules in two. `claude plugin validate` reports impersonation and the reserved
 * names of the "Validation messages" table; "the exact official marketplace names ... pass
 * validation" and are refused when the marketplace is added or loaded, as are the other spellings,
 * the JavaScript member names, the `claudeai-` prefix and the download-folder clash. So there are
 * two functions: `validationNameIssues` (what `validate` prints) and `loadNameIssues` (what adding
 * or loading refuses, which `marketplaceNameIssues` adds to the first).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/marketplace_names.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { MarketplaceSource } from './types.ts';

const call = coreModule('format/marketplace/marketplace-names');

/** Which reserved name a trailing dot or a symbol in place of a hyphen spells ("v2.1.280"), if any. */
export const spellingOf = (name: string): string | undefined => call('spellingOf', name);

/** What `claude plugin validate` reports about a marketplace's name beyond the id grammar. */
export const validationNameIssues = (name: string): PluginIssue[] =>
  call('validationNameIssues', name);

export interface NameOrigin {
  /** Where the marketplace came from; only a `github` or `git` source under `github.com/anthropics/` may use an official name. */
  readonly origin?: MarketplaceSource;
  /** The marketplace is hosted on claude.ai, the only one that may start with `claudeai-`. */
  readonly hostedOnClaudeAi?: boolean;
}

/** What adding or loading a marketplace refuses about its name, on top of `validationNameIssues`. */
export const loadNameIssues = (name: string, where: NameOrigin = {}): PluginIssue[] =>
  call('loadNameIssues', name, where);

/** Every refusal a marketplace's name can draw: `validate`'s and adding or loading's. */
export const marketplaceNameIssues = (name: string, where: NameOrigin = {}): PluginIssue[] =>
  call('marketplaceNameIssues', name, where);

/** A registered marketplace, as far as the download-folder rule needs it. */
export interface RegisteredMarketplace {
  readonly name: string;
  readonly source: MarketplaceSource;
}

/**
 * Adding a marketplace named like the download folder of another registered GitHub marketplace is
 * refused while that marketplace is registered under a different name (requires Claude Code
 * v2.1.290 or later).
 */
export const downloadFolderIssues = (
  name: string,
  registered: readonly RegisteredMarketplace[],
): PluginIssue[] => call('downloadFolderIssues', name, registered);
