import { coreModule } from '../../wasm/bridge.ts';
import type { SourceParseResult } from './types.ts';

/**
 * Reading a marketplace source object (plugins/marketplace-reference.md, "Marketplace sources",
 * "Fields by type", "Source values valid only in policy lists", "Source objects in settings"): the
 * `source` of an `extraKnownMarketplaces` value, or an entry of `strictKnownMarketplaces` and
 * `blockedMarketplaces`. `policy` says which of the three keys the object is in. Issue paths are
 * relative to the source object (`repo`, `plugins[0].name`); the caller prefixes its own.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/marketplace_source.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/marketplace/marketplace-source');

export interface MarketplaceSourceOptions {
  /** The object is an entry of `strictKnownMarketplaces` or `blockedMarketplaces`. */
  readonly policy: boolean;
  /** The `extraKnownMarketplaces` key; a `settings` source's `name` must equal it. */
  readonly key?: string;
}

/** What the `path` of a `github` or `git` source defaults to ("Fields by type"). */
export const DEFAULT_MARKETPLACE_PATH: '.claude-plugin/marketplace.json' = call(
  'DEFAULT_MARKETPLACE_PATH',
);

/**
 * The marketplace source an object describes. `loadStatus` is what `extraKnownMarketplaces` does
 * with it, whichever key it is in: a type Claude Code parses can still fail to load, and an object
 * that matches no source type (`source` is undefined) loads nothing. Never throws.
 */
export const parseMarketplaceSource = (
  raw: unknown,
  options: MarketplaceSourceOptions,
): SourceParseResult => call('parseMarketplaceSource', raw, options);
