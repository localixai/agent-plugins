/**
 * How far a plugin got (plugins/loading.md, "Check which stage a plugin reached"; plugins/
 * overview.md, "Make an installed plugin available in your session"): an `enabledPlugins` entry is
 * **declared** in settings, **fetched** to disk (`installed_plugins.json` records it and `cache/`
 * holds its files), and **loaded** in the running session, which "changes to settings or to disk
 * don't reach ... until you run `/reload-plugins` or start a new session". Pure over facts the host
 * reads: the settings, the records, which install paths exist, and the set the session loaded.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/stage_report.rs`,
 * docs/RUST.md).
 */
import type { MarketplaceSource } from '../format/marketplace/types.ts';
import type { ErrorsTabEntry } from './install-messages.ts';
import type { InstallRecord } from './installed-plugins.ts';
import type { KnownMarketplaces } from './known-marketplaces.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/stage-report');

export interface StageFacts {
  /** `<name>@<marketplace>`. */
  readonly id: string;
  /** Settings declare the plugin on (`enabledPlugins`, after precedence). */
  readonly enabled: boolean;
  /** Its install records, as `installed_plugins.json` lists them. */
  readonly records: readonly InstallRecord[];
  /** Which of the records' `installPath` exist on disk. */
  readonly existing: ReadonlySet<string>;
  /** A relative-path plugin of a marketplace added from a local path: it loads in place and needs no record. */
  readonly inPlace?: boolean;
  /** The plugin is in the set the session loaded at startup or at the last `/reload-plugins`. */
  readonly loaded: boolean;
}

export type Stage = 'none' | 'declared' | 'fetched' | 'loaded';

/** The three layers of overview.md: settings, disk and session. */
export type Layer = 'settings' | 'disk' | 'session';

export interface StageReport {
  readonly declared: boolean;
  /** `installed_plugins.json` has a record for it (or it loads in place). */
  readonly recorded: boolean;
  /** A directory its record points at exists (or it loads in place). */
  readonly cached: boolean;
  readonly loaded: boolean;
  /** The furthest stage reached. */
  readonly stage: Stage;
  /** Settings or disk differ from the session: `/reload-plugins` or a new session applies the change. */
  readonly reload: boolean;
  /** The layers it is not present at; empty when it is available in the session. */
  readonly missing: readonly Layer[];
  readonly available: boolean;
  /** `Plugin "<name>" not cached at <path>`, for an enabled plugin whose record points at nothing. */
  readonly problem?: ErrorsTabEntry;
}

export const stageReport = (facts: StageFacts): StageReport => call('stageReport', facts);

// ---- after the session starts ----

/** What Claude Code does after the session starts for a marketplace that settings declare. */
export type StartupAction =
  | {
      readonly marketplace: string;
      readonly action: 'clone';
      readonly source: MarketplaceSource;
      /** Next, plugins reload and enabled plugins that are not cached yet are downloaded. */
      readonly next: 'reload-and-download-enabled';
    }
  | {
      readonly marketplace: string;
      readonly action: 'refetch';
      readonly source: MarketplaceSource;
      readonly next: 'notify';
      readonly notice: string;
    };

/**
 * "Plugins load at session start from `installed_plugins.json` and the cache without using the
 * network. After the session starts, Claude Code checks the declared marketplaces in the
 * background": one `known_marketplaces.json` lacks is cloned (then it reloads plugins and downloads
 * the enabled ones not cached yet), and one whose source changed in settings is re-fetched from the
 * new source with `Plugins changed. Run /reload-plugins to activate.` Sources match as in
 * `sameMarketplaceSource`. A known marketplace that settings do not declare is not touched.
 */
export const planMarketplaceStartup = (
  declared: Readonly<Record<string, MarketplaceSource>>,
  known: KnownMarketplaces,
): readonly StartupAction[] => call('planMarketplaceStartup', declared, known);

/**
 * The `/plugin` Errors tab entries after the start: an enabled plugin that neither the clone nor
 * the re-fetch brought in (`fetchedAfterStart`) and that has no usable cache directory shows
 * `Plugin "<name>" not cached at <path>`.
 */
export const unresolvedAfterStart = (
  plugins: readonly StageFacts[],
  fetchedAfterStart: ReadonlySet<string>,
): readonly ErrorsTabEntry[] => call('unresolvedAfterStart', plugins, fetchedAfterStart);
