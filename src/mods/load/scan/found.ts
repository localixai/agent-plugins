import type { ModScanIssue } from './issue-codes.ts';
import type { EngineFold } from './engine-fold.ts';
import type { ScannedCall, ScannedHook } from './uses.ts';

/** What the scan of one file's hooks collects as it walks them. */
export interface Found {
  readonly hooks: ScannedHook[];
  readonly calls: Map<string, ScannedCall>;
  readonly envReads: Set<string>;
  readonly envWrites: Set<string>;
  readonly stateReads: Set<string>;
  readonly stateWrites: Set<string>;
  /** The hosts `$.http.fetch` and `$.audio.play` calls spell, and the methods called with a URL the scan cannot read. */
  readonly hostsNamed: Set<string>;
  readonly hostsUnread: Set<string>;
  /** The literal paths `$.wasm.compile` calls name, each with the line of its first call. */
  readonly wasm: Map<string, number>;
  readonly issues: ModScanIssue[];
  /** What the `engine.create` hooks in the form add to and withhold from `$` (`engineFold`). */
  readonly folds: EngineFold[];
}

export const emptyFound = (): Found => ({
  hooks: [],
  calls: new Map(),
  envReads: new Set(),
  envWrites: new Set(),
  stateReads: new Set(),
  stateWrites: new Set(),
  hostsNamed: new Set(),
  hostsUnread: new Set(),
  wasm: new Map(),
  issues: [],
  folds: [],
});
