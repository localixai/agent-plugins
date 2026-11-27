import type { Clock } from '../ports/clock.ts';
import type { PluginStateStore, Sha256, StateStoreFailure } from '../ports/marketplace.ts';
import type { InstalledPluginsRead } from './installed-plugins.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Reading `installed_plugins.json` through `PluginStateStore` and doing what the troubleshooting
 * page says Claude Code does with a file it cannot use (plugins/troubleshooting.md): keep the old
 * text of an unreadable file in a dated `.kept` copy and start the file over; copy records under
 * names no version can use into a dated `.set-aside` file, then drop them; and never rewrite the file
 * while a record, or the format, is one this version cannot read. A copy is written before the file
 * changes, so a failed copy loses nothing.
 *
 * Judgments (spec/judgments.md): `<date>` is the clock's UTC day as `YYYY-MM-DD`; `<hash>` is the
 * first eight hex digits of the SHA-256 of the text the copy keeps; a `.set-aside` copy is the
 * dropped keys with their raw values, as JSON.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/installed_plugins_store.rs`,
 * docs/RUST.md).
 */

const call = coreModule('marketplace/installed-plugins-store');

export interface InstalledPluginsPorts {
  readonly store: PluginStateStore;
  readonly sha256: Sha256;
  readonly clock: Clock;
}

export interface LoadedInstalledPlugins {
  readonly read: InstalledPluginsRead;
  /** A copy or the rewritten file could not be written: the file is as it was. */
  readonly failure?: StateStoreFailure;
}

/** Reads the records and applies the recovery the read calls for. A name here is relative to the root. */
export const loadInstalledPlugins = (
  ports: InstalledPluginsPorts,
  { signal }: { readonly signal: AbortSignal },
): Promise<LoadedInstalledPlugins> => call.async('loadInstalledPlugins', ports, { signal });
