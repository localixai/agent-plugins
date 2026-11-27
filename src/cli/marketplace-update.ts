import type { CommandSurface } from '../marketplace/install-plans.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput } from './result.ts';

const call = coreModule('cli/marketplace-update');

/**
 * `claude plugin marketplace update [name]` as a plan (plugins/cli-reference.md, "plugin marketplace
 * update"; plugins/install.md, "Update plugins now"; plugins/troubleshooting.md, "Marketplace is
 * seed-managed"): which marketplaces' listings are refreshed, from what, and what is printed. It
 * refreshes listings only (`planUpdate`'s `marketplace-listing`): installed plugins keep their
 * versions. In a session the line is `✔ Updated 1 marketplace` (plugins/troubleshooting.md, "Install
 * a plugin"). Pure; the host fetches. Judgment (spec/judgments.md): a session counts every listing
 * it refreshed the way a shell does, `✔ Updated N marketplaces`, 0 for an empty registry. After a
 * git refresh that left LFS files as pointer files the host adds `lfsPointerLine(count)`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/marketplace_update.rs`, docs/RUST.md).
 */

export interface UpdateArguments {
  /** Absent: every marketplace. */
  readonly name?: string;
  readonly json?: boolean;
  /** Where the command was typed; absent: a shell. */
  readonly surface?: CommandSurface;
}

export interface RegisteredForUpdate {
  readonly name: string;
  /** The branch or tag it was added with. */
  readonly ref?: string;
  /** `CLAUDE_CODE_PLUGIN_SEED_DIR` supplied it: the directory; it is read-only. */
  readonly seeded?: string;
}

export interface UpdateFacts {
  readonly registered: readonly RegisteredForUpdate[];
}

export interface RefreshTarget {
  readonly name: string;
  /** "Updates to the latest commit of that ref, not the repository's default branch." */
  readonly ref?: string;
}

export interface MarketplaceUpdatePlan {
  readonly output: CommandOutput;
  readonly refresh: readonly RefreshTarget[];
  /** Plugin ids updated: always none, "leaves your installed plugins at their current versions". */
  readonly update: readonly string[];
}

/** What `plugin marketplace update` refreshes and prints. */
export const planMarketplaceUpdate = (
  args: UpdateArguments,
  facts: UpdateFacts,
): MarketplaceUpdatePlan => call('planMarketplaceUpdate', args, facts);
