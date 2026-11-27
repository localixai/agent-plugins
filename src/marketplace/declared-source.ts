/**
 * Adding a marketplace whose `name` already has an `extraKnownMarketplaces` entry in user or managed
 * settings (plugins/troubleshooting.md, "Cannot add marketplace "<name>": its source doesn't match its
 * extraKnownMarketplaces entry in user or managed settings"): the add is refused and registers nothing
 * unless the sources match. Pure; the host reads the entry.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/declared_source.rs`,
 * docs/RUST.md).
 */
import type { MarketplaceSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/declared-source');

/** The text the docs quote since v2.1.287. */
export const declaredSourceMismatch = (name: string): string =>
  call('declaredSourceMismatch', name);

export interface DeclaredSourceFacts {
  readonly name: string;
  /** The source of the add, as `parseMarketplaceInput` built it. */
  readonly given: MarketplaceSource;
  /** The source the user or managed `extraKnownMarketplaces` entry lists for that name; absent when none does. */
  readonly declared?: MarketplaceSource;
}

/**
 * "Two sources match when they have the same type and the same value in every field. An entry that sets a
 * `ref` you didn't pass counts as different. A `github` entry also counts as different when you gave the
 * repository as an `https://github.com/` URL, because Claude Code records that URL as a `git` source."
 * `undefined` when there is nothing to refuse.
 */
export const declaredSourceRefusal = (facts: DeclaredSourceFacts): string | undefined =>
  call('declaredSourceRefusal', facts);

/** The fix the docs give first: "Use the declared source", typed as the entry writes it. */
export const declaredSourceFix = (declared: MarketplaceSource): string | undefined =>
  call('declaredSourceFix', declared);
