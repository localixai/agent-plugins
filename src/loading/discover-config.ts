/**
 * The sources of a plugin's JSON configuration (hooks, LSP servers, monitors), its `settings.json`
 * and its `bin/` (plugins/manifest-reference.md, "Standard layout").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/discover_config.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { ConfigInput, DiscoverContext, Discovered, Found } from './discovered.ts';

const call = coreModule('loading/discover-config');

/** The kinds whose files are JSON configuration, read by their own reader. */
export const CONFIG_KINDS: readonly ['hooks', 'lspServers', 'monitors'] = call('CONFIG_KINDS');
export type ConfigKind = (typeof CONFIG_KINDS)[number];

/**
 * The sources of hooks, LSP servers or monitors in loading order: the default file and what the
 * manifest declares, combined by the kind's rule (`hooks` and `lspServers` merge, the default
 * first; `monitors` is replaced by `experimental.monitors`, the top-level `monitors` alias used
 * only when it is absent: judgment). A default file that is not there is skipped.
 */
export const discoverConfig = (
  kind: ConfigKind,
  context: DiscoverContext,
): Promise<Found<ConfigInput>> => call.async('discoverConfig', kind, context);

/** The `settings.json` at the plugin root, when it is there and inside the plugin. */
export const discoverSettings = (context: DiscoverContext): Promise<Found<string>> =>
  call.async('discoverSettings', context);

/** `bin/`: whether it exists (a plugin "that has this directory") and the files directly in it. */
export const discoverBin = (context: DiscoverContext): Promise<Discovered['bin']> =>
  call.async('discoverBin', context);
