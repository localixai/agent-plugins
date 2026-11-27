/**
 * `known_marketplaces.json`, "each marketplace Claude Code has fetched, with its `source`,
 * `installLocation`, `lastUpdated`, and `autoUpdate`", one per user (plugins/loading.md, "Check
 * which stage a plugin reached"); where a marketplace is on disk ("Find plugins on disk"); and the
 * two ways the registry fails to load (errors.md, "Failed to load marketplace configuration").
 *
 * The SDK types the `source` of an `extraKnownMarketplaces` entry, with `installLocation` and
 * `autoUpdate`; the file adds `lastUpdated`. The kit's `MarketplaceSource` names its member
 * `kind` (`directory`, `file`); the file writes that under `source`, as the settings key does. A
 * host that kept its registry in another shape migrates it to this one itself, once: the kit reads
 * and writes only the shape above and keeps no reader for another.
 * Judgments (spec/judgments.md): the wording of the corrupted-file message after its quoted lead;
 * a registry with any entry that does not match is read as empty, since every command that needs it fails.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/known_marketplaces.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { MarketplaceSource } from '../format/marketplace/types.ts';
import type { StoreLayout } from './store-layout.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/known-marketplaces');

export interface KnownMarketplace {
  readonly source: MarketplaceSource;
  /** `marketplaces/<name>/`, or for a local `file` or `directory` source the path you gave. */
  readonly installLocation: string;
  /** When Claude Code last fetched it, as the file writes it (ISO text). */
  readonly lastUpdated: string;
  /** Whether the marketplace and its installed plugins update on startup; absent means the default. */
  readonly autoUpdate?: boolean;
}

export type KnownMarketplaces = Readonly<Record<string, KnownMarketplace>>;

/** `unreadable`: "isn't valid JSON or can't be read" (an empty file too); `corrupted`: valid JSON that does not match the schema. */
export type RegistryFailureCode = 'unreadable' | 'corrupted';

export interface KnownMarketplacesRead {
  readonly marketplaces: KnownMarketplaces;
  readonly failure?: { readonly code: RegistryFailureCode; readonly message: string };
  readonly issues: readonly PluginIssue[];
}

/** The registry's text; no file is no marketplaces. */
export const parseKnownMarketplaces = (text: string | undefined): KnownMarketplacesRead =>
  call('parseKnownMarketplaces', text);

/** The registry's text, two-space indented with a final newline (a judgment, as for `installed_plugins.json`). */
export const serializeKnownMarketplaces = (marketplaces: KnownMarketplaces): string =>
  call('serializeKnownMarketplaces', marketplaces);

/**
 * Where the registry says a marketplace is: `marketplaces/<name>/` for one added from GitHub,
 * another Git host or a URL; for a local `file` or `directory` source "no copy here", so the path
 * you gave (plugins/loading.md, "Find plugins on disk"). Judgment (spec/judgments.md): the other
 * kinds, which the docs do not place, are kept like a clone. The layout joins with the host's
 * function and answers at once, so its folder is asked here and the core picks.
 */
export const installLocationFor = (
  source: MarketplaceSource,
  name: string,
  layout: StoreLayout,
): string => call('installLocationFor', source, layout.marketplace(name));

/**
 * "Two sources match when they have the same type and the same value in every field" (plugins/
 * troubleshooting.md, "already added from a different source"): an entry that sets a `ref` you
 * did not pass differs, and so does a `github` entry from the same repository as an https URL
 * (recorded as `git`). Field order does not matter; list order does.
 */
export const sameMarketplaceSource = (a: MarketplaceSource, b: MarketplaceSource): boolean =>
  call('sameMarketplaceSource', a, b);
