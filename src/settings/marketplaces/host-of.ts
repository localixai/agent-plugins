/**
 * The host a `hostPattern` entry is matched against (settings-reference.md, `strictKnownMarketplaces`
 * "Allowed source types", `hostPattern`): the hostname of a `url` source, and of a `git` source
 * depending on the form of the git URL.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/host_of.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/marketplaces/host-of');

/** The hostname of a `url` source (judgment: lower case, as the URL parser spells it). */
export const hostOfUrl = (address: string): string | undefined => call('hostOfUrl', address);

/**
 * The host of a `git` source's URL. `wide` is the blocklist: it also reads an address with no
 * scheme and no `user@`. Anything else has no host (`undefined`).
 */
export const hostOfGit = (address: string, wide: boolean): string | undefined =>
  call('hostOfGit', address, wide);
