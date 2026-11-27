import type { MarketplaceSourceOf } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { Rendered } from './result.ts';

const call = coreModule('cli/marketplace-list');

/**
 * `claude plugin marketplace list` (plugins/cli-reference.md, "plugin marketplace list"): the text
 * listing and the `--json` array, from the marketplaces the host reads out of its registry. Pure.
 * The layout of a text entry is the kit's (spec/judgments.md); the JSON fields are the docs'.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/marketplace_list.rs`, docs/RUST.md).
 */

/** A marketplace hosted on claude.ai has no local clone. */
export interface ClaudeAiSource {
  readonly kind: 'claudeai';
  readonly marketplaceId: string;
  readonly organizationUuid: string;
}

/** The sources the docs' `source` field names: `github`, `git`, `url`, `directory`, `file` and `claudeai`. */
export type ListedSource =
  | MarketplaceSourceOf<'github' | 'git' | 'url' | 'directory' | 'file'>
  | ClaudeAiSource;

export interface ConfiguredMarketplace {
  readonly name: string;
  readonly source: ListedSource;
  /** Where Claude Code cached it; a claude.ai marketplace has none. */
  readonly installLocation?: string;
  /** A claude.ai entry's recorded scope. */
  readonly scope?: string;
  /** A claude.ai entry's status. */
  readonly status?: string;
}

/** A marketplace claude.ai lists for the account that is not added yet. */
export interface ClaudeAiListing {
  readonly name: string;
}

export interface ListRequest {
  readonly marketplaces: readonly ConfiguredMarketplace[];
  readonly fromClaudeAi?: readonly ClaudeAiListing[];
}

/** Every field is a string. */
export type ListedJson = Readonly<Record<string, string>>;

export interface ListPlan {
  /** The `--json` array: configured marketplaces only. */
  readonly json: readonly ListedJson[];
  /** The text lines. */
  readonly text: readonly string[];
  /** What the command prints: the array as the one line with `--json`, else the text; it exits 0. */
  print(json: boolean): Rendered;
}

/** The listing: `--json` covers "configured marketplaces only and leaves the section out". */
export const planMarketplaceList = (request: ListRequest): ListPlan =>
  call('planMarketplaceList', request);
