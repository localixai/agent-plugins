import type { PluginIssue } from '../format/issues.ts';
import type { MarketplaceSourceKind } from '../format/marketplace/types.ts';
import type { Clock } from '../ports/clock.ts';
import type { Environment } from '../ports/environment.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * When Claude Code reads a marketplace again before it looks a plugin up for an install
 * (plugins/loading.md, "When Claude Code refreshes a marketplace before an install"). Pure: the
 * host fetches; the kit says which marketplaces, which are skipped and why, and what a failure
 * leaves. Every fact comes in as data, the clock and the environment through their ports.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/updates/refresh_before_install.rs`,
 * docs/RUST.md).
 */
const call = coreModule('updates/refresh-before-install');

/** "You can run `/plugin install` in a session or `claude plugin install` in your shell." */
export const INSTALL_COMMANDS: readonly ['/plugin install', 'claude plugin install'] =
  call('INSTALL_COMMANDS');
export type InstallCommand = (typeof INSTALL_COMMANDS)[number];

/** What the kit needs to know about a marketplace the user has registered. */
export interface KnownMarketplace {
  readonly name: string;
  /** How it was added: `file` and `directory` are local paths, `settings` is defined inline. */
  readonly source: MarketplaceSourceKind;
  /** Its resolved auto-update setting (`marketplaceAutoUpdate`). */
  readonly autoUpdate: boolean;
  /** A seed directory supplies it (`CLAUDE_CODE_PLUGIN_SEED_DIR`). */
  readonly seeded?: boolean;
  /** When the host last refreshed it, in `Clock.now()` milliseconds. */
  readonly lastRefreshedAt?: number;
  /** The marketplace gate's message when managed settings block it ("is blocked by enterprise policy"). */
  readonly blocked?: string;
}

export interface RefreshRequest {
  /** `name` or `name@marketplace`, as the user typed it. */
  readonly ref: string;
  readonly command: InstallCommand;
  readonly marketplaces: readonly KnownMarketplace[];
  /** The lookup of a bare name in the cached catalogs found no plugin. */
  readonly lookupMissed: boolean;
  readonly env: Environment;
  readonly clock: Clock;
}

/** Why nothing is refreshed. */
export const SKIP_REASONS: readonly [
  'invalid-reference',
  'marketplace-unknown',
  'local-or-inline-source',
  'seed-directory',
  'refreshed-within-30-seconds',
  'nonessential-traffic-disabled',
  'cli-reads-cached-catalogs',
  'lookup-found-the-plugin',
  'no-auto-update-marketplace',
  'every-candidate-skipped',
] = call('SKIP_REASONS');
export type SkipReason = (typeof SKIP_REASONS)[number];

export type RefreshDecision =
  | { readonly outcome: 'refresh'; readonly marketplaces: readonly string[] }
  | { readonly outcome: 'skip'; readonly reason: SkipReason }
  /** "Managed settings block the marketplace, in which case Claude Code also refuses the install." */
  | {
      readonly outcome: 'refuse';
      readonly reason: 'blocked-by-managed-settings';
      readonly issue: PluginIssue;
    };

/**
 * What to refresh before the lookup of `request.ref`. The refresh of a named marketplace depends
 * neither on its `autoUpdate` nor on `DISABLE_AUTOUPDATER`: only the five skips and the managed
 * block above apply.
 */
export const refreshBeforeInstall = (request: RefreshRequest): RefreshDecision =>
  call('refreshBeforeInstall', request);

export interface RefreshFailure {
  readonly installFrom: 'cached catalog';
  /** The CLI's report; the docs name none for `/plugin install`. */
  readonly report?: PluginIssue;
}

/**
 * "When the refresh fails, the install proceeds from the cached catalog and `claude plugin install`
 * reports `marketplace not refreshed`."
 */
export const refreshFailed = (command: InstallCommand): RefreshFailure =>
  call('refreshFailed', command);
