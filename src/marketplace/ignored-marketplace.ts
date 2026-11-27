/**
 * `Marketplace "<name>" is added but ignored` (plugins/troubleshooting.md): an entry of
 * `known_marketplaces.json` that fails a check made every time the file is read, so the
 * marketplace "and the plugins installed from it stop loading". The checks are the ones the
 * section lists: the location, a git URL, an http(s) URL, a match with the `extraKnownMarketplaces`
 * entry of user or managed settings, and the name. Which location is on a network drive is the
 * host's to probe (an effect); everything else is decided here.
 *
 * Judgments (spec/judgments.md): the docs quote the line and the fix only for the location check,
 * so the fix for the others is the kit's wording; the name is refused when `loadNameIssues` finds an
 * issue with the entry's own source (another spelling of a reserved name, a member name of
 * JavaScript objects), and a reserved name refused only for its source is the untrusted-source case
 * (`untrustedSource`), not this one; a git URL is unusable when `readGitAddress` refuses
 * it or its host is loopback, link-local or a cloud-metadata address (`isBlockedHost`); the Errors
 * tab line ends with the reason's period.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/ignored_marketplace.rs`,
 * docs/RUST.md).
 */
import type { MarketplaceSource } from '../format/marketplace/types.ts';
import type { ErrorsTabEntry } from './install-messages.ts';
import type { KnownMarketplace } from './known-marketplaces.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/ignored-marketplace');

/** What the host found out about the entry's location (the clone's folder, or the local path it was added from). */
export interface IgnoreChecks {
  /** `network`: on a network drive; `unchecked`: it could not be examined; `local`: a path on this computer. */
  readonly location: 'local' | 'network' | 'unchecked';
  /** The `extraKnownMarketplaces` source of the same name in user or managed settings (not a repository's). */
  readonly declared?: MarketplaceSource;
}

export interface IgnoredReason {
  /** The sentence after `is added but ignored`; absent when only the debug log has the reason. */
  readonly sentence?: string;
  readonly fix: string;
}

/**
 * `Marketplace "<name>" is registered from an untrusted source: ...` for an entry registered under a
 * reserved name from a source that is not an `anthropics` GitHub repository: it and its plugins
 * stop loading. `undefined` for any other entry. The host checks this on every load beside
 * `ignoredReason`, which leaves this case to it.
 */
export const untrustedSource = (name: string, entry: KnownMarketplace): string | undefined =>
  call('untrustedSource', name, entry);

/**
 * Why an entry is ignored, or `undefined` when it loads: the first check that fails, in the order
 * the docs list them (location, git URL or URL, settings declaration, name).
 */
export const ignoredReason = (
  name: string,
  entry: KnownMarketplace,
  checks: IgnoreChecks,
): IgnoredReason | undefined => call('ignoredReason', name, entry, checks);

/** Where the line shows: `claude plugin list` (one line, name bare) or the `/plugin` Errors tab (name quoted, fix beneath). */
export type IgnoredSurface = 'list' | 'errors-tab';

/** The line `claude plugin list` prints (text), or the Errors tab entry (message and guidance). */
export function ignoredLine(name: string, reason: IgnoredReason, surface: 'list'): string;
export function ignoredLine(
  name: string,
  reason: IgnoredReason,
  surface: 'errors-tab',
): ErrorsTabEntry;
export function ignoredLine(
  name: string,
  reason: IgnoredReason,
  surface: IgnoredSurface,
): string | ErrorsTabEntry {
  return call('ignoredLine', name, reason, surface);
}

/**
 * `Marketplace "<name>" is registered from an untrusted source: ...` (troubleshooting.md): a
 * marketplace registered under a reserved name whose source is not an `anthropics` GitHub
 * repository stops loading with its plugins. The middle sentence is the name refusal for that
 * source (Agent Plugins Extended §13.20 item 2): for a source that is not GitHub or git it reads
 * `can only be used with GitHub sources from the 'anthropics' organization`. Without a source, the
 * GitHub sentence.
 */
export const untrustedSourceMessage = (name: string, source?: MarketplaceSource): string =>
  call('untrustedSourceMessage', name, source);
