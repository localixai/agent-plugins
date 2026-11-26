import type { PluginIssue } from '../format/issues.ts';
import type { PluginManifest } from '../format/manifest-schema.ts';
import type { MarketplaceEntry } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { DeclaredDependency } from './dependency-graph.ts';

const call = coreModule('marketplace/dependency-catalog');

/**
 * The `dependencies` of a `CatalogPlugin`: its `plugin.json`'s, then its marketplace entry's, in the
 * order `mergeEntryWithManifest` lists them, each with where it was declared, since a refused
 * cross-marketplace dependency refuses the install when the entry declares it and is left out when
 * `plugin.json` does (plugins/dependencies.md, "Depend on a plugin from another marketplace").
 * Bare names resolve against `marketplace`, the declaring plugin's own (manifest-reference.md,
 * `dependencies`). `manifest` is `undefined` for a plugin with no `plugin.json`, whose entry is the
 * manifest, and for one not fetched yet (then the host also sets `manifestUnread`). An item that does
 * not read is an issue at its place in its list and is left out.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/dependency_catalog.rs`, docs/RUST.md).
 */
export const catalogDependencies = (
  entry: MarketplaceEntry,
  marketplace: string,
  manifest: PluginManifest | undefined,
): {
  readonly dependencies: readonly DeclaredDependency[];
  readonly issues: readonly PluginIssue[];
} => call('catalogDependencies', entry, marketplace, manifest);
