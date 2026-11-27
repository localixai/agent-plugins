/**
 * Where a matching plugin is suggested and how often (plugins/relevance.md, "Understand how plugin
 * relevance works", "Preview what the user sees"): the spinner tip, the session-start notification
 * and the `/plugin` Discover tab. Pure over what the host recorded; the texts are the docs'. The
 * counting of "sessions" and the annotations the docs do not quote are in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/relevance_surface.rs`,
 * docs/RUST.md).
 */
import type { Relevance } from '../format/marketplace/types.ts';
import type { SignalName } from './relevance-match.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/relevance-surface');

export interface SuggestedPlugin {
  readonly name: string;
  readonly marketplace: string;
  readonly relevance?: Relevance;
}

/** "Working with Terraform? Install the terraform-helpers plugin:" and the install command on the next line. */
export const spinnerTipText = (plugin: SuggestedPlugin): string => call('spinnerTipText', plugin);

/** The one-line notification of a `cwd` match at session start. */
export const sessionStartNoticeText = (plugin: SuggestedPlugin): string =>
  call('sessionStartNoticeText', plugin);

/** What the host recorded about one plugin's suggestions. */
export interface SuggestionHistory {
  /** Sessions started since the spinner tip or the notification last showed it; `undefined` when never shown. */
  readonly sessionsSinceShown?: number;
  /** Times the spinner tip and the notification together have shown it. */
  readonly timesShown: number;
  /** The Discover tab already pinned it once (`~/.claude.json`). */
  readonly discoverPinned: boolean;
}

export interface SuggestionContext {
  readonly installed: boolean;
  /** The signals that match now (`matchingSignals`). */
  readonly matched: readonly SignalName[];
  readonly history: SuggestionHistory;
  /** `spinnerTipsEnabled`; unset is on. */
  readonly spinnerTipsEnabled?: boolean;
  /** A `spinnerTipsOverride` with `excludeDefault` replaces the built-in tips. */
  readonly tipsOverrideExcludesDefault?: boolean;
}

/** Whether the spinner tip shows the plugin now. */
export const showSpinnerTip = (context: SuggestionContext): boolean =>
  call('showSpinnerTip', context);

/** Whether the session-start notification shows the plugin: a `cwd` match, the spacing, and at most two showings. */
export const showSessionStartNotice = (context: SuggestionContext): boolean =>
  call('showSessionStartNotice', context);

/** The Discover tab pins the plugin the first time the tab opens while a signal matches; neither tip setting affects it. */
export const pinInDiscover = (context: SuggestionContext): boolean =>
  call('pinInDiscover', context);

/** The annotation that names the first matching signal; `command` is the matching command name of a `cli` signal. */
export const discoverAnnotation = (
  matched: readonly SignalName[],
  command?: string,
): string | undefined => call('discoverAnnotation', matched, command);
