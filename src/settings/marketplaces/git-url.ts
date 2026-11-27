/**
 * Canonical identity of a git repository address, the comparison `blockedMarketplaces` makes
 * ("Git URLs are canonicalized, so the `git@` and `https://` forms, `.git` suffixes, and trailing
 * slashes of one `github.com` repository all match the same entry", plugins/org.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/git_url.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/marketplaces/git-url');

export interface RepoId {
  readonly host: string;
  readonly path: string;
}

export const canonicalRepo = (address: string): RepoId | undefined =>
  call('canonicalRepo', address);
