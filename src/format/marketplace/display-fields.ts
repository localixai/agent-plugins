/**
 * The display fields users see in plugin listings and details (plugins/marketplace-reference.md,
 * "Display fields"): `displayName`, `description`, `author`, `homepage`, `repository`, `license` and
 * `keywords`. "For a field you set on the entry, users see the entry's value, even when `plugin.json`
 * sets a different one. For a field the entry leaves unset, users see the `plugin.json` value."
 * Pure over a parsed entry and, when the host can read it, the plugin's `plugin.json`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/display_fields.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginManifest } from '../manifest-schema.ts';
import type { MarketplaceEntry } from './types.ts';

const call = coreModule('format/marketplace/display-fields');

export type DisplayFields = Readonly<
  Partial<
    Pick<
      PluginManifest,
      'displayName' | 'description' | 'author' | 'homepage' | 'repository' | 'license' | 'keywords'
    >
  >
>;

/** Whether the listing is of a plugin not installed yet (`listed`) or one whose files are on disk (`installed`). */
export type ListingStage = 'listed' | 'installed';

/**
 * The display fields of one plugin as users see them at `stage`. `manifest` is the plugin's own
 * `plugin.json` (`undefined` when it has none or the host has not read it); before install it is
 * used only for a relative-path entry, whose files are inside the marketplace.
 */
export const listingFields = (
  entry: MarketplaceEntry,
  manifest: PluginManifest | undefined,
  stage: ListingStage,
): DisplayFields => call('listingFields', entry, manifest, stage);
