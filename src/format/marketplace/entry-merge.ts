import type { PluginIssue } from '../issues.ts';
import type { PluginManifest } from '../manifest-schema.ts';
import type { MarketplaceEntry } from './types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * How a plugin entry and the plugin's own `plugin.json` combine into the manifest Claude Code loads
 * (plugins/marketplace-reference.md, "How an entry combines with plugin.json", "Hooks in an entry",
 * "Display fields", "Strict mode"; plugins/manifest-reference.md, "How entry fields combine with
 * `plugin.json`", "Metadata precedence"). Pure over a parsed entry and a parsed `plugin.json`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/entry_merge.rs`,
 * docs/RUST.md).
 */
const call = coreModule('format/marketplace/entry-merge');

/** What loading the plugin from this entry gives. */
export interface EntryMerge {
  /**
   * The manifest the plugin loads with, or `undefined` when the plugin fails to load (the strict
   * conflict). Its presence, not the issues' severity, says whether the plugin loads: the
   * `not yet supported in a marketplace entry` error leaves the plugin loading without those hooks.
   */
  readonly manifest: PluginManifest | undefined;
  /** The entry name: the install id and the `enabledPlugins` key. `manifest.name` is the namespace. */
  readonly installName: string;
  /**
   * Hook events the entry declares and the manifest also holds: "the entry's matchers for an event
   * replace the manifest's matchers for that same event". Hook files the manifest names (and
   * `hooks/hooks.json`) are not read here, so the loader drops their matchers for these events.
   */
  readonly replacedHookEvents: readonly string[];
  /** False when the plugin's default `skills/` directory is not scanned (root-source entry that lists skills). */
  readonly scanDefaultSkills: boolean;
  readonly issues: readonly PluginIssue[];
}

/**
 * The manifest a plugin loads with, given its marketplace entry and its own `plugin.json`
 * (`undefined` when it has none).
 */
export const mergeEntryWithManifest = (
  entry: MarketplaceEntry,
  manifest: PluginManifest | undefined,
): EntryMerge => call('mergeEntryWithManifest', entry, manifest);

/**
 * The name users see: the display name the merge settled on (the entry's, then plugin.json's), "when
 * neither ... sets one, users see the plugin's `name`" (marketplace-reference.md, `displayName`),
 * read as the name of the merged manifest (spec/judgments.md), the entry name when it fails to load.
 */
export const effectiveDisplayName = (merge: EntryMerge): string =>
  call('effectiveDisplayName', merge);
