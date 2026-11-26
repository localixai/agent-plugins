import { coreModule } from '../wasm/bridge.ts';
import type { PluginManifest } from './manifest-schema.ts';

/**
 * The standard layout (plugins/manifest-reference.md, "Standard layout"): the default location of
 * each component under the plugin root, the manifest key that points elsewhere, and how that key
 * combines with the default ("How each key combines with its default location"). Discovery reads
 * this registry and nothing else, so a component kind added here fails the typecheck wherever it
 * needs handling.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/layout.rs`, docs/RUST.md).
 */
const call = coreModule('format/layout');

export const COMPONENT_KINDS: readonly [
  'manifest',
  'skills',
  'commands',
  'agents',
  'hooks',
  'mcpServers',
  'lspServers',
  'outputStyles',
  'workflows',
  'themes',
  'monitors',
  'bin',
  'settings',
] = call('COMPONENT_KINDS');
export type ComponentKind = (typeof COMPONENT_KINDS)[number];

/** `replace`: the key replaces the default. `add`: both load. `merge`: the default loads first, the key merges into it. */
export const COMBINE_MODES: readonly ['replace', 'add', 'merge'] = call('COMBINE_MODES');
export type CombineMode = (typeof COMBINE_MODES)[number];

/**
 * A manifest key that names a component, written as the docs write it (`experimental.themes`).
 * Reading it is the core's (`declaredKey`): a function never crosses the boundary with the table.
 */
export interface ManifestKey {
  readonly key: string;
}

export interface LayoutEntry {
  /** Relative to the plugin root, `/`-separated, no `./` and no trailing slash. */
  readonly location: string;
  /** Whether the default location is a directory (false: a file). */
  readonly directory: boolean;
  /**
   * The folder the "Default <folder>/ folder is ignored" warning names, for a key that replaces the
   * default; `undefined` for a kind that cannot be ignored.
   */
  readonly folder: string | undefined;
  /** The manifest keys that point elsewhere, the first one set wins. */
  readonly manifestKeys: readonly ManifestKey[];
  /** `undefined`: the kind has no path key that combines with a default (manifest, bin, settings). */
  readonly combine: CombineMode | undefined;
  /** The extension a scan of a directory of this kind keeps, `undefined` for any. */
  readonly extension: string | undefined;
}

export const STANDARD_LAYOUT: { readonly [K in ComponentKind]: LayoutEntry } =
  call('STANDARD_LAYOUT');

/** The first manifest key of a kind that is set, with its value; `undefined` when none is. */
export const declaredKey = (
  manifest: PluginManifest | undefined,
  kind: ComponentKind,
): { readonly key: string; readonly value: unknown } | undefined =>
  call('declaredKey', manifest, kind);

/**
 * The component kinds a directory listing shows at the plugin root, by their default locations (the
 * manifest's `.claude-plugin/` folder is not a component). `names` are the entry names of the root.
 */
export const componentsAt = (names: readonly string[]): readonly ComponentKind[] =>
  call('componentsAt', names);
