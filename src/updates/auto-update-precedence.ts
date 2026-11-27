import type { PluginIssue } from '../format/issues.ts';
import type { Environment } from '../ports/environment.ts';
import type { PolicyScope } from '../settings/layers.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Which marketplaces auto-update, and whether the pass runs at all (plugins/loading.md, "Which
 * marketplaces and plugins auto-update"; plugins/install.md, "Turn auto-update on or off for a
 * marketplace"; plugins/org.md, "Turn auto-update on or off per marketplace" and "Turn updates off
 * for the whole fleet"; plugins/host-marketplace.md, "Turn on auto-update"). Pure: the host reads
 * the files, the kit decides.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/updates/auto_update_precedence.rs`, docs/RUST.md).
 */

const call = coreModule('updates/auto-update-precedence');

export interface AutoUpdateFacts {
  readonly name: string;
  /** `autoUpdate` on the marketplace's `extraKnownMarketplaces` entry in the settings file that wins; unset is `undefined`. */
  readonly settingsEntry?: boolean;
  /** `autoUpdate` on its `known_marketplaces.json` entry, which the Enable auto-update toggle writes. */
  readonly recorded?: boolean;
  /** The marketplace was added from claude.ai. */
  readonly fromClaudeAi?: boolean;
  /** A seed directory supplies it: "forces `autoUpdate` off for seed marketplaces" (plugins/org.md). */
  readonly seeded?: boolean;
}

/**
 * Whether the marketplace auto-updates: "the first of these that is set": the settings entry, the
 * `known_marketplaces.json` entry, the default. A seed marketplace is off before any of them
 * (judgment on the order, spec/judgments.md: the docs say the seed "forces" it off).
 */
export const marketplaceAutoUpdate = (facts: AutoUpdateFacts): boolean =>
  call('marketplaceAutoUpdate', facts);

/**
 * Whether the pass runs, and the Enable auto-update toggle shows: `DISABLE_UPDATES=1`,
 * `DISABLE_AUTOUPDATER=1` or any non-empty `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` turns both
 * off "unless you also set `FORCE_AUTOUPDATE_PLUGINS=1`".
 */
export const autoUpdatePassAllowed = (env: Environment): boolean =>
  call('autoUpdatePassAllowed', env);

/** What the Marketplaces tab labels the toggle: "Enable auto-update" when it is off, "Disable auto-update" when it is on. */
export const toggleLabel = (facts: AutoUpdateFacts): string => call('toggleLabel', facts);

/** The settings file that declares the marketplace under `extraKnownMarketplaces`, as the registry plans it. */
export interface Declaration {
  readonly scope: PolicyScope;
  readonly autoUpdate?: boolean;
}

export interface ToggleRequest {
  readonly name: string;
  /** What the user chose: Enable (true) or Disable (false). */
  readonly enable: boolean;
  readonly env: Environment;
  readonly declared?: Declaration;
}

export type ToggleWrite =
  | { readonly file: 'known_marketplaces.json' }
  | { readonly file: 'settings'; readonly scope: PolicyScope };

export type ToggleDecision =
  | { readonly outcome: 'hidden' }
  | { readonly outcome: 'refused'; readonly issue: PluginIssue }
  | {
      readonly outcome: 'write';
      readonly autoUpdate: boolean;
      readonly writes: readonly ToggleWrite[];
    };

/**
 * The Enable or Disable auto-update toggle: hidden when the pass is off; refused when a managed
 * entry sets the field; otherwise it writes `known_marketplaces.json`, and the settings entry too
 * "when a settings file also declares the marketplace under `extraKnownMarketplaces`" (only a file
 * the user can write, judgment). A managed entry that leaves the field unset keeps the user toggle.
 */
export const toggleDecision = (request: ToggleRequest): ToggleDecision =>
  call('toggleDecision', request);
