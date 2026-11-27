import { coreModule } from '../wasm/bridge.ts';
import type { GraphFacts } from './dependency-graph.ts';
import type { InstallScope } from './install-scopes.ts';

/**
 * Pruning and uninstalling (plugins/cli-reference.md, `plugin prune`: "Remove auto-installed
 * dependencies that no installed plugin needs anymore. The command never removes a plugin you
 * installed yourself"; `plugin uninstall --prune`: "Also remove auto-installed dependencies that no
 * remaining plugin needs"; plugins/install.md, "Plugins with dependencies": "auto-installed
 * dependencies stay until you run `claude plugin prune`"; plugins/dependencies.md, "Combine
 * constraints from several plugins": "When a user uninstalls the last plugin that constrains a
 * dependency, the dependency ... resumes tracking its marketplace entry"). Pure over `GraphFacts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/dependency_prune.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/dependency-prune');

/**
 * The auto-installed plugins at `scope` that no remaining plugin needs, transitive orphans
 * included. A plugin the user installed is a root: never removed, and everything it needs, directly
 * or through other dependencies, stays. Judgment: only plugins installed at the same scope count as
 * needing, since a dependency is installed "at the same scope" as its dependent; a user-installed
 * plugin that is disabled still needs its dependencies (it is installed); dependencies that only
 * each other need are orphans.
 */
export const orphanedAutoInstalled = (facts: GraphFacts, scope: InstallScope): readonly string[] =>
  call('orphanedAutoInstalled', facts, scope);

/** What prune shows and does, by the flags and the terminal (cli-reference.md, `plugin prune`). */
export interface PruneRun {
  readonly dryRun?: boolean;
  readonly yes?: boolean;
  /** Both stdin and stdout are a terminal. */
  readonly tty: boolean;
}

export type PruneOutcome =
  | { readonly kind: 'nothing'; readonly message: string }
  | { readonly kind: 'listed'; readonly orphans: readonly string[]; readonly message: string }
  | { readonly kind: 'ask'; readonly orphans: readonly string[]; readonly prompt: string }
  | { readonly kind: 'refused'; readonly orphans: readonly string[]; readonly message: string }
  | { readonly kind: 'removed'; readonly orphans: readonly string[]; readonly message: string };

/**
 * `claude plugin prune` as a decision (cli-reference.md, `plugin prune`): with none to remove a line
 * that starts `Nothing to prune`; `--dry-run` lists them and ends `(dry run — nothing removed)`;
 * `-y` removes them and prints `Removed N auto-installed plugins: <names>`; an interactive terminal
 * without `-y` asks `Remove? [y/N]`; a pipe without `-y` lists them, prints
 * ``Not a TTY — run `claude plugin prune -y` to remove.`` and removes nothing.
 */
export const planPrune = (facts: GraphFacts, scope: InstallScope, run: PruneRun): PruneOutcome =>
  call('planPrune', facts, scope, run);

export interface UninstallPlan {
  /** The plugin removed from `scope`. */
  readonly remove: string;
  /** With `--prune`: the auto-installed dependencies no remaining plugin needs, removed with it. */
  readonly pruned: readonly string[];
  /**
   * The dependencies the plugin constrained that no other installed plugin constrains any more:
   * each resumes tracking its marketplace entry on the next update.
   */
  readonly released: readonly string[];
}

/**
 * `claude plugin uninstall <id> [--prune]` as a decision: the plugin goes; its auto-installed
 * dependencies stay unless `prune` is set, then those no remaining plugin needs go too (transitive
 * ones included). Judgment: uninstalling a plugin other plugins depend on is not refused, since the
 * docs state no such check for uninstall (they do for disable); the dependents then fail to load
 * with `is not installed`.
 */
export const planUninstall = (
  id: string,
  scope: InstallScope,
  facts: GraphFacts,
  prune: boolean,
): UninstallPlan => call('planUninstall', id, scope, facts, prune);
