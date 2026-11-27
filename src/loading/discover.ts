/**
 * Finding a plugin folder's components through `PluginFiles` alone: the default locations and the
 * manifest's paths, nothing parsed.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/discover.rs`, docs/RUST.md).
 */
import type { PluginManifest } from '../format/manifest-schema.ts';
import type { PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { Discovered } from './discovered.ts';
import type { LocateOptions } from '../format/locate.ts';

const call = coreModule('loading/discover');

/**
 * What decides whether a declared path leaves the plugin, the signal of the reads, and whether the
 * default `skills/` is scanned (`EntryMerge.scanDefaultSkills`; default true).
 */
export type DiscoverOptions = LocateOptions & { readonly scanDefaultSkills?: boolean };

/**
 * Finds every component of a plugin folder (plugins/manifest-reference.md, "Standard layout", "Path
 * rules", "How each key combines with its default location") through `PluginFiles` alone: the
 * default locations and the manifest's paths, each key combined with its default by its own rule,
 * containment and existence checked, nothing parsed. `pluginName` is what components are namespaced
 * under and what names a skill at the plugin root; `locateContext` sets the platform default.
 */
export const discoverComponents = (
  files: PluginFiles,
  manifest: PluginManifest | undefined,
  pluginName: string,
  options: DiscoverOptions = {},
): Promise<Discovered> => call.async('discoverComponents', files, manifest, pluginName, options);
