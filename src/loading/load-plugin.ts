/**
 * Loading a plugin folder into a typed `Plugin`: one crossing per load, the host's files a port the
 * core asks through one load's view of it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/load_plugin.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { PluginManifest } from '../format/manifest-schema.ts';
import type { LinkRoots } from '../format/path-escape.ts';
import type { CollectOptions } from '../format/mcp-sources.ts';
import type { PluginFiles } from '../ports/index.ts';
import type { MarketplaceEntry } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { LoadedPlugin } from './plugin-model.ts';

const call = coreModule('loading/load-plugin');

export interface LoadPluginInput {
  /** Read access to the plugin folder: its root is the plugin root. */
  readonly files: PluginFiles;
  /**
   * The name to load a plugin without a manifest under: "the plugin name then comes from the marketplace
   * entry, or from the directory name when you load the plugin with `--plugin-dir`"
   * (manifest-reference.md, "Manifest file"). The manifest's `name` wins when there is one.
   */
  readonly fallbackName?: string;
  /** Opens `.mcpb` and `.dxt` bundles named by `mcpServers`; without it a bundle is a warning. */
  readonly bundles?: CollectOptions['bundles'];
  /** The platform the host runs on; a backslash in a declared path escapes the plugin on `posix`. Defaults to `posix`. */
  readonly platform?: 'posix' | 'windows';
  /** Where the plugin and its marketplace really are, so a link to another plugin of the marketplace loads. */
  readonly roots?: LinkRoots;
  /**
   * The marketplace entry the plugin is installed from (`readMarketplace`), its `strict` included: the
   * plugin loads with the manifest `mergeEntryWithManifest` gives (plugins/manifest-reference.md, "How
   * entry fields combine with `plugin.json`"). Its name loads a plugin without `plugin.json`, ahead of
   * `fallbackName`. Absent, the plugin's own `plugin.json` is the manifest (`--plugin-dir`).
   */
  readonly entry?: MarketplaceEntry;
  readonly signal?: AbortSignal;
}

export interface ManifestStep {
  /** Whether `.claude-plugin/plugin.json` exists. */
  readonly present: boolean;
  readonly manifest: PluginManifest | undefined;
  readonly issues: readonly PluginIssue[];
}

/** Reads the optional manifest: absent is fine, unreadable JSON or a manifest with an error is not. */
export const loadManifest = (files: PluginFiles, signal?: AbortSignal): Promise<ManifestStep> =>
  call.async('loadManifest', files, signal);

/**
 * Loads a plugin folder into a typed `Plugin` (docs/API.md, "Loading"): the optional manifest, every
 * component in the standard layout and the manifest's paths, each reader's issues collected. Never
 * throws. The plugin is `undefined` only when it does not load at all: a plugin folder or manifest
 * that cannot be checked or leads outside (`manifestGate`), a manifest that is not valid
 * JSON or fails validation, no name to load it under, a guard hook that cannot be read or an inline
 * MCP server of the manifest that cannot be read (oracle 2.1.319, spec/judgments.md). Any other problem (a path that is missing or
 * escapes the plugin, a file Claude Code would skip) is an issue and skips that component, as the
 * docs say for paths ("the plugin loads without that component"); the rest loads (spec/judgments.md).
 */
export const loadPlugin = (input: LoadPluginInput): Promise<LoadedPlugin> =>
  call.async('loadPlugin', input);
