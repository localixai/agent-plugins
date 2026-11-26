import { coreModule } from '../wasm/bridge.ts';

/**
 * How a seed's marketplaces enter the registry (plugins/org.md, "Seed containers and CI"): "Claude
 * Code registers the marketplace from these directories at startup"; "Claude Code uses the first seed
 * that contains a given marketplace"; "a marketplace declared in the seed overwrites the user's entry
 * of the same name"; the seed is read-only and "forces `autoUpdate` off for seed marketplaces".
 * Pure: the host reads `known_marketplaces.json` of each directory and applies the result.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/seed_registry.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/seed-registry');

/** What the kit needs of a registry entry. */
export interface SeededEntry {
  readonly autoUpdate?: boolean;
}

/** One seed directory and the marketplaces its `known_marketplaces.json` declares. */
export interface Seed<E extends SeededEntry> {
  readonly directory: string;
  readonly marketplaces: Readonly<Record<string, E>>;
}

export interface RegistryEntry<E extends SeededEntry> {
  readonly entry: E;
  /** The seed directory that supplies it; the entry is read-only while this is set. */
  readonly seedDirectory?: string;
}

/**
 * The registry after the seeds: every user entry, replaced where a seed declares the same name, and
 * every seed marketplace the user has no entry for. `seeds` is in the order of the path list.
 */
export const applySeeds = <E extends SeededEntry>(
  user: Readonly<Record<string, E>>,
  seeds: readonly Seed<E>[],
): Readonly<Record<string, RegistryEntry<E>>> => call('applySeeds', user, seeds);
