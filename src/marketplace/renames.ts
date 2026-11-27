import type { Marketplace } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { InstallScope } from './install-scopes.ts';
import type { RenameStatus } from './types.ts';

/**
 * What a marketplace's `renames` map does to a user who still has an old plugin name (plugins/
 * host-marketplace.md, "Migrate users with a renames map"). Pure decisions over data: the host
 * reads and writes the settings, and shows each notice.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/renames.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/renames');

/**
 * Where `name` stands in `marketplace`: `current` when it is listed (a listed name wins over a key
 * of the same name, judgment); otherwise the chain from the oldest name, followed to the plugin it
 * ends at (`renamed`) or `null` (`removed`); a name the map does not hold, and a chain that does not
 * resolve, are `unknown`, the case that is `Plugin "<name>" not found in marketplace`.
 */
export const resolveRename = (name: string, marketplace: Marketplace): RenameStatus =>
  call('resolveRename', name, marketplace);

/** The notice a user sees for `name` in `marketplace`, or nothing when it is current or unknown. */
export const renameNotice = (name: string, marketplace: Marketplace): string | undefined =>
  call('renameNotice', name, marketplace);

export interface MigratedKeys<V> {
  readonly map: Record<string, V>;
  readonly notices: readonly string[];
}

/**
 * Rewrites the keys of one `enabledPlugins` or `pluginConfigs` map for the marketplace registered
 * as `marketplaceName`: an old name's key becomes the new name's with its value unchanged, the key
 * of a removed plugin is dropped, every other key stays. When the user already has a key under the
 * new name, theirs is kept and the old one dropped (judgment). The notices are those the user sees,
 * each once. When two old keys are rewritten to the same new key and the user has none, the first
 * in map order wins and the other is dropped (judgment). The given map is not changed.
 */
export const migrateSettingsKeys = <V>(
  map: Readonly<Record<string, V>>,
  marketplaceName: string,
  marketplace: Marketplace,
): MigratedKeys<V> => call('migrateSettingsKeys', map, marketplaceName, marketplace);

export type ScopedSettings<V> = { readonly [S in InstallScope]?: Readonly<Record<string, V>> };

export interface MigratedScopes<V> {
  readonly scopes: ScopedSettings<V>;
  readonly notices: readonly string[];
}

/**
 * `migrateSettingsKeys` over the maps of each scope: user, project and local are rewritten; managed
 * settings cannot be, so they come back as given and their notice is given again on every call
 * until an admin updates them. A notice that several scopes give is given once.
 */
export const migrateSettingsScopes = <V>(
  scopes: ScopedSettings<V>,
  marketplaceName: string,
  marketplace: Marketplace,
): MigratedScopes<V> => call('migrateSettingsScopes', scopes, marketplaceName, marketplace);
