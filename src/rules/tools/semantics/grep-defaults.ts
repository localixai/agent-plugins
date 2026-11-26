import { coreModule } from '../../../wasm/bridge.ts';
import type { GrepOutputMode } from '../inputs/grep.ts';

/**
 * What Grep does by default (tools-reference.md, "Grep tool behavior").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/grep_defaults.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/tools/semantics/grep-defaults');

/**
 * The output mode a Grep call runs in. A value that is not one of the three is read as absent here;
 * `parseToolInput` reports it (spec/judgments.md).
 */
export const effectiveGrepOutputMode = (input: unknown): GrepOutputMode =>
  call('effectiveGrepOutputMode', input);

/**
 * tools-reference.md, "Grep tool behavior": "By default, patterns match within a single line.
 * Claude can set `multiline: true`"; the SDK's GrepInput says "Default: false". Only `true` turns
 * it on (spec/judgments.md).
 */
export const effectiveGrepMultiline = (input: unknown): boolean =>
  call('effectiveGrepMultiline', input);

export interface GrepWindowRequest {
  /** "Skip first N lines/entries before applying head_limit"; 0 when absent. */
  readonly offset?: number | undefined;
  /** "Limit output to first N lines/entries"; 250 when absent, 0 for no limit. */
  readonly head_limit?: number | undefined;
}

export interface GrepCount {
  readonly path: string;
  readonly count: number;
}

export interface GrepWindow {
  readonly lines: readonly string[];
  /** "No entries at this offset" when the offset is past the last of a pattern that has matches. */
  readonly notice: string | undefined;
}

export const NO_ENTRIES_AT_OFFSET: 'No entries at this offset' = call('NO_ENTRIES_AT_OFFSET');

/**
 * The lines Grep returns in `content` mode (tools-reference.md, "Grep tool behavior"): the window
 * `offset` and `head_limit` select. "When the tool's `offset` parameter points past the last match
 * for a pattern that has matches, Grep returns `No entries at this offset`"; a pattern with no
 * match has no such notice (spec/judgments.md).
 */
export const windowGrepContent = (
  matches: readonly string[],
  request: GrepWindowRequest,
): GrepWindow => call('windowGrepContent', matches, request);

/** "A total across all matching files" that "covers every match" however the listing is cut. */
export const totalGrepCount = (counts: readonly GrepCount[]): number =>
  call('totalGrepCount', counts);

/** The per-file entries of `count` mode after `offset` and `head_limit`, with the whole total. */
export const windowGrepCounts = (
  counts: readonly GrepCount[],
  request: GrepWindowRequest,
): { readonly entries: readonly GrepCount[]; readonly total: number } =>
  call('windowGrepCounts', counts, request);
