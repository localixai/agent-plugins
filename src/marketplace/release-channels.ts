/**
 * Release channels (plugins/host-marketplace.md, "Run release channels"): "Claude Code has no
 * release-channel concept". A channel is a marketplace whose entries point at another ref of the same
 * plugin; the two `marketplace.json` files need different `name` values, because "a user can't have
 * two marketplaces with the same name registered at once", and the two refs need different versions,
 * or none, because "updates are detected by comparing versions, so a ref that moves without a version
 * change leaves users on the cached copy". This checks the pair as a hosting author would before
 * publishing; the issues are findings for the author, not a refusal Claude Code makes.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/release_channels.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { Marketplace } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/release-channels');

/** The `version` of each plugin's `plugin.json` at the ref a channel points at, by plugin name. It wins over the entry's version. */
export type ChannelManifestVersions = ReadonlyMap<string, string>;

export interface ChannelManifests {
  readonly first?: ChannelManifestVersions;
  readonly second?: ChannelManifestVersions;
}

/**
 * What is wrong with `first` and `second` as the two catalogs of a channel pair. Judgments
 * (spec/judgments.md): the same name is an error and the rest are warnings; only plugins both list
 * are compared; an unset version on either side is fine (the SHA tells them apart); two entries that
 * point at the same ref and sha are reported, since they are not two channels.
 */
export const checkReleaseChannels = (
  first: Marketplace,
  second: Marketplace,
  manifests: ChannelManifests = {},
): readonly PluginIssue[] => call('checkReleaseChannels', first, second, manifests);
