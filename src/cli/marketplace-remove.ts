/**
 * `claude plugin marketplace remove <name>` as a plan (plugins/cli-reference.md, "plugin marketplace
 * remove"): which settings files lose the declaration and, when the last one does, the cache, the
 * plugins installed from it and their options, secrets and data. Pure over what the host read; the
 * host deletes. `--scope` has no `-s`. From the last scope it also "removes their `enabledPlugins`
 * entries from your settings files" (plugins/install.md, "Remove a marketplace"): "you installed" and
 * "your settings files" are why `managed` is left alone. The Marketplaces tab names the `uninstall`
 * ids before it asks to confirm. The wording for a marketplace nobody knows and for one plugin, and
 * what a plugin only managed settings installed does, are in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/marketplace_remove.rs`, docs/RUST.md).
 */
import type { ChosenScope, InstallScope } from '../marketplace/install-scopes.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput } from './result.ts';
import type { ConfigScope } from './scope.ts';

const call = coreModule('cli/marketplace-remove');

export interface RemoveArguments {
  /** The marketplace name `plugin marketplace list` shows, not the source passed to `add`. */
  readonly name: string;
  readonly scope?: string;
  readonly json?: boolean;
}

/** A plugin installed from the marketplace and the scopes it is installed at. */
export interface InstalledFromMarketplace {
  /** `name@marketplace`. */
  readonly id: string;
  readonly scopes: readonly InstallScope[];
}

export interface RemoveFacts {
  /** The marketplace is registered (on disk or declared). */
  readonly known: boolean;
  /** The settings files that declare it. */
  readonly declaredIn: readonly ConfigScope[];
  readonly plugins: readonly InstalledFromMarketplace[];
  /** Every `enabledPlugins` entry in the user, project and local settings files. */
  readonly enabledEntries: readonly EnabledEntry[];
  /** `CLAUDE_CODE_PLUGIN_SEED_DIR` supplies it: the seed directory; it is read-only. */
  readonly seeded?: string;
}

/** One `enabledPlugins` key in one settings file. */
export interface EnabledEntry {
  /** `name@marketplace`. */
  readonly id: string;
  readonly scope: ChosenScope;
}

/**
 * One plugin uninstalled because its marketplace is gone: "Also deletes their saved options and
 * secrets and data where it can". The flags are the docs' three deletions, every one on because the
 * plugin leaves its last scope; `--keep-data` and the cases that keep the data directory belong to
 * `plugin uninstall`.
 */
export interface PluginRemoval {
  readonly id: string;
  /** The scopes to uninstall from: every scope but `managed`, which the user cannot change. */
  readonly scopes: readonly InstallScope[];
  readonly deleteOptions: true;
  readonly deleteSecrets: true;
  readonly deleteData: true;
}

export interface RemovePlan {
  readonly output: CommandOutput;
  /** The settings files to remove the declaration from. */
  readonly removeFrom: readonly ConfigScope[];
  /** The marketplace was removed from the last scope that declares it. */
  readonly deleteCache: boolean;
  readonly uninstall: readonly PluginRemoval[];
  /** The `enabledPlugins` entries of the marketplace's plugins to delete, when it leaves its last scope. */
  readonly dropEnabledEntries: readonly EnabledEntry[];
}

/** What `plugin marketplace remove` does and prints. */
export const planMarketplaceRemove = (args: RemoveArguments, facts: RemoveFacts): RemovePlan =>
  call('planMarketplaceRemove', args, facts);
