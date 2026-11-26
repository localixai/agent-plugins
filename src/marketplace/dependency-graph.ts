/**
 * The facts the dependency graph reads, as plain data (plugins/dependencies.md; plugins/install.md,
 * "Plugins with dependencies"; plugins/cli-reference.md, `plugin enable`, `plugin disable`,
 * `plugin prune`). The kit has no disk and no settings reader: a host (or the settings and updates
 * modules) reads `installed_plugins.json`, the `enabledPlugins` of each scope and the marketplace
 * catalog, and passes what it found. Every function of the graph is pure over these facts.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/dependency_graph.rs`, docs/RUST.md).
 */
import type { PluginSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { DeclaredIn } from './cross-marketplace.ts';
import type { DependencyIssue, InstalledDependency } from './dependencies.ts';
import type { InstallScope } from './install-scopes.ts';
import type { NormalizedDependency } from './types.ts';

const call = coreModule('marketplace/dependency-graph');

/** A dependency as a manifest or a marketplace entry declares it: where it was declared decides what a cross-marketplace refusal does. */
export interface DeclaredDependency {
  readonly dep: NormalizedDependency;
  readonly declaredIn: DeclaredIn;
  /** The item names a marketplace itself (`namesMarketplace(item)`); absent means it does not, which a `--plugin-dir` copy meets on every Claude Code version. */
  readonly namesMarketplace?: boolean;
}

/** One install of a plugin at one scope (`installed_plugins.json`). */
export interface InstalledRecord {
  /** `name@marketplace`. */
  readonly id: string;
  readonly scope: InstallScope;
  /** Installed as another plugin's dependency, not by the user: only such a plugin is pruned. */
  readonly autoInstalled?: boolean;
  /** The version constraints are checked against: the tag's for a tag-resolved copy, else the `plugin.json` version. */
  readonly version?: string;
  /** The plugin's `defaultEnabled` (the entry's over `plugin.json`'s, `mergeEntryWithManifest`); absent is `true`. */
  readonly defaultEnabled?: boolean;
  /** The plugin's own dependencies, normalized. */
  readonly dependencies: readonly NormalizedDependency[];
}

/** `enabledPlugins` of each scope: the value written for a plugin id, if any. */
export type EnabledPlugins = Readonly<
  Partial<Record<InstallScope, Readonly<Record<string, boolean>>>>
>;

/** A plugin loaded for one session with `--plugin-dir`: it is `<name>@inline`, whatever its manifest says. */
export interface LocalCopy {
  readonly name: string;
  readonly enabled: boolean;
  readonly version?: string;
}

/** What the marketplace catalog says about a plugin the dependency graph may have to install. */
export interface CatalogPlugin {
  readonly name: string;
  readonly marketplace: string;
  readonly source: PluginSource;
  /** The dependencies of its marketplace entry and of its `plugin.json`, each with where it was declared (`catalogDependencies`). */
  readonly dependencies: readonly DeclaredDependency[];
  /**
   * Its `plugin.json` is not read yet, so `dependencies` holds the entry's only: before install,
   * Claude Code reads `plugin.json` only for a relative-path source (marketplace-reference.md,
   * "Display fields"). An install plan that reaches it lists it in `needs`; the host fetches it,
   * fills `dependencies` (and `currentCopyVersion`) from its `plugin.json`, drops the flag and plans again.
   */
  readonly manifestUnread?: true;
  /** The tags of the repository that hosts the plugin; none for a source with no tags or a folder that is not a git repository. */
  readonly tags?: readonly string[];
  /** The commit each tag names (by tag name), for the 12-character suffix `claude plugin list` shows. */
  readonly tagCommits?: Readonly<Record<string, string>>;
  /** The `version` in the `plugin.json` of the marketplace's current copy. */
  readonly currentCopyVersion?: string;
  /** The entry sets a `headersHelper`: Claude Code never installs it as a dependency. */
  readonly headersHelper?: string;
  /** The plugin's `defaultEnabled` (the entry's over `plugin.json`'s); absent is `true`. */
  readonly defaultEnabled?: boolean;
}

export interface GraphFacts {
  readonly installed: readonly InstalledRecord[];
  readonly enabled: EnabledPlugins;
  /** The catalog by plugin id (`name@marketplace`). */
  readonly catalog: Readonly<Record<string, CatalogPlugin>>;
  /** `allowCrossMarketplaceDependenciesOn` of each marketplace, by its name; only the root's is read. */
  readonly allowlists: Readonly<Record<string, readonly string[]>>;
  /** Plugin ids the organization's plugin policy blocks. */
  readonly blocked?: ReadonlySet<string>;
  readonly localCopies?: readonly LocalCopy[];
}

export const recordAt = (
  id: string,
  scope: InstallScope,
  facts: GraphFacts,
): InstalledRecord | undefined => call('recordAt', id, scope, facts);

/** The highest-precedence scope whose `enabledPlugins` lists `id`, if any does. */
export const winningScope = (id: string, facts: GraphFacts): InstallScope | undefined =>
  call('winningScope', id, facts);

/** The value the winning scope writes for `id`, if any scope writes one. */
export const effectiveEnabled = (id: string, facts: GraphFacts): boolean | undefined =>
  call('effectiveEnabled', id, facts);

/**
 * Whether an installed plugin is on: the value the winning scope writes, else its `defaultEnabled`
 * ("Default `true`. Whether the plugin starts enabled when the user hasn't set it in
 * `enabledPlugins`", marketplace-reference.md). A plugin with no install record is not on.
 */
export const isEnabled = (id: string, facts: GraphFacts): boolean => call('isEnabled', id, facts);

/** The dependencies a plugin declares, from any of its installed records (the manifest does not differ per scope). */
export const dependenciesOf = (id: string, facts: GraphFacts): readonly NormalizedDependency[] =>
  call('dependenciesOf', id, facts);

/** An installed plugin that constrains `dependency` with a range. */
export interface Constraint {
  readonly range: string;
  /** The id of the constraining plugin. */
  readonly by: string;
}

/**
 * The ranges installed plugins put on a dependency (plugins/dependencies.md, "Combine constraints
 * from several plugins"), once per constraining plugin. Judgment: every installed plugin counts,
 * whatever its scope or whether it is enabled; the docs say "installed plugins".
 */
export const constraintsOn = (dependency: string, facts: GraphFacts): readonly Constraint[] =>
  call('constraintsOn', dependency, facts);

/** What "installed on a host that loads the copy" means for the `--plugin-dir` copy of a dependency. */
export interface CopyLookup {
  /** The Claude Code version being emulated; an entry that names a marketplace matches a local copy from 2.1.242. Absent means a current one. */
  readonly hostVersion?: string;
  /** The entry names a marketplace itself (`namesMarketplace(item)`), not merely resolves to its plugin's. */
  readonly namesMarketplace?: boolean;
}

export const localCopyOf = (
  dep: NormalizedDependency,
  facts: GraphFacts,
  lookup: CopyLookup = {},
): InstalledDependency | undefined => call('localCopyOf', dep, facts, lookup);

/**
 * The copy that meets a dependency entry (plugins/dependencies.md, "Test a plugin and its
 * dependency locally"): the one installed from the marketplace, else the `--plugin-dir` copy of the
 * same name. The local copy meets an entry with no marketplace and, from Claude Code 2.1.242, one
 * that names a marketplace; no range is checked against it. Judgment: an installed copy wins over a
 * local one, as "until you install the dependency from its marketplace" puts the local copy first
 * only until then.
 */
export const findDependencyCopy = (
  dep: NormalizedDependency,
  facts: GraphFacts,
  lookup: CopyLookup = {},
): InstalledDependency | undefined => call('findDependencyCopy', dep, facts, lookup);

/** The load-time check of one declared dependency against what is installed or loaded locally. */
export const checkDeclaredDependency = (
  dep: NormalizedDependency,
  facts: GraphFacts,
  lookup: CopyLookup = {},
): DependencyIssue | undefined => call('checkDeclaredDependency', dep, facts, lookup);

/** What `claude plugin disable` does with a plugin other plugins still need. */
export type DisableDecision =
  | { readonly allowed: true }
  | {
      readonly allowed: false;
      /** The enabled plugins that declare the target directly. */
      readonly dependents: readonly string[];
      /** Every plugin to disable, in the order that works: dependents of dependents first, the target last. */
      readonly chain: readonly string[];
      /** The chained command that disables them in that order. */
      readonly command: string;
      readonly message: string;
    };

/** The enabled plugins that declare `id` as a dependency. */
export const dependentsOf = (id: string, facts: GraphFacts): readonly string[] =>
  call('dependentsOf', id, facts);

/**
 * Whether a plugin may be disabled (plugins/install.md, "Plugins with dependencies": "when another
 * enabled plugin still needs the one you named, Claude Code refuses and prints a chained command
 * that disables both in the right order"; cli-reference.md, `plugin disable`: "names the dependents
 * to disable first"). The refusal text is a judgment; the docs quote none.
 */
export const disableDecision = (id: string, facts: GraphFacts): DisableDecision =>
  call('disableDecision', id, facts);
