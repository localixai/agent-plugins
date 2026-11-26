/**
 * A plugin's `dependencies` (plugins/dependencies.md, "Declare a dependency with a version
 * constraint"): an entry is `"name"`, `"name@marketplace"` or an object whose `name` (required),
 * `version` and `marketplace` are strings. The marketplace is the declaring plugin's own unless set;
 * the `version` is a node-semver range. This is the single owner of that reading; `loading` and the
 * entry validators import it. Checks of what is installed are in `checkInstalledDependency` and
 * `checkDependency`; tags and versions are in `dependency-tags.ts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/dependencies.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { NormalizedDependency } from './types.ts';

const call = coreModule('marketplace/dependencies');

/** `name@marketplace`, or `name` when the dependency has no marketplace (a plugin loaded from claude.ai names none). */
export const dependencyId = (dep: NormalizedDependency): string => call('dependencyId', dep);

/**
 * Reads one `dependencies` item. Judgments (spec/judgments.md): an empty or blank `version` is no
 * constraint (node-semver reads "" as any version); a key other than the three is ignored. The
 * `name` and the marketplace must each satisfy the plugin id grammar, as a marketplace entry's name
 * must, or they could never resolve. A string with two `@` and an object without `name` are
 * `Invalid input` on the item, as the manifest reports them (spec J16.17).
 * `declaringMarketplace` is the marketplace of the plugin that declares it, or `undefined` for a
 * plugin that has none (loaded from claude.ai), whose bare names then have no marketplace.
 */
export const normalizeDependency = (
  value: unknown,
  declaringMarketplace: string | undefined,
  path?: string,
): NormalizedDependency | PluginIssue =>
  call('normalizeDependency', value, declaringMarketplace, path);

/**
 * Whether a `dependencies` item names a marketplace itself (`"name@marketplace"`, or an object with
 * `marketplace`), as opposed to resolving to the declaring plugin's. Normalizing hides it, and it
 * decides whether a `--plugin-dir` copy meets the entry on Claude Code before 2.1.242.
 */
export const namesMarketplace = (value: unknown): boolean => call('namesMarketplace', value);

/** A dependency as the host found it: its version, whether it is enabled, and whether it is a `--plugin-dir` copy. */
export interface InstalledDependency {
  /** The tag's version for a tag-resolved copy, else the `version` of its `plugin.json`. */
  readonly version?: string;
  readonly enabled: boolean;
  /** A copy loaded with `--plugin-dir`: it satisfies the entry and no range is checked against it. */
  readonly local?: boolean;
}

/**
 * The `errorDetails.type` `claude plugin list --json` carries for a dependency problem
 * (plugins/troubleshooting.md, "Dependency errors"): `dependency-unsatisfied` for a dependency that
 * is not installed (also with the `@synced` note) or disabled, `dependency-version-unsatisfied` for
 * `Requires "<dep>" <range>, installed <version>`.
 */
export type DependencyErrorType = 'dependency-unsatisfied' | 'dependency-version-unsatisfied';

/** What is wrong with the dependency: no copy meets it, its copy is off, or its version is outside the range. */
export type DependencyCause = 'missing' | 'disabled' | 'out-of-range';

/**
 * A dependency problem, with the type the host reports it under and the facts a host words its own
 * advice from. Judgment (spec/judgments.md): `message` is the documented text; Claude Code 2.1.322
 * adds `— run \`claude plugin install <dep>\`, or check that its marketplace is added` to the
 * `is not installed` line, advice for its own CLI, which the kit leaves to the host.
 */
export interface DependencyIssue extends PluginIssue {
  readonly errorType: DependencyErrorType;
  /** The declared dependency: `name@marketplace`, or `name` when it names no marketplace. */
  readonly dependency: string;
  readonly cause: DependencyCause;
}

/**
 * Whether a declared dependency is met by its installed copy (plugins/troubleshooting.md,
 * "Dependency errors"): `Dependency "<dep>" is not installed`, `... is disabled`, and
 * `Requires "<dep>" <range>, installed <version>`. A `--plugin-dir` copy meets the entry whatever
 * its version. The dependency is named as the entry declares it, a disabled `--plugin-dir` copy too:
 * not `<name>@inline` (spec J16.24, oracle 2.1.325).
 */
export const checkInstalledDependency = (
  dep: NormalizedDependency,
  installed: InstalledDependency | undefined,
): DependencyIssue | undefined => call('checkInstalledDependency', dep, installed);

/** The two places a dependency that names a marketplace can be found. */
export interface DependencyCopies {
  /** Installed from the marketplace the entry names. */
  readonly marketplace?: InstalledDependency;
  /** The claude.ai plugin of the same name, `<name>@synced`. */
  readonly claudeAi?: InstalledDependency;
}

/**
 * Whether a dependency is met given both places it can be (plugins/dependencies.md, "Dependencies
 * when your plugin loads from claude.ai"). A plugin loaded from claude.ai (`loadsFromClaudeAi`) has
 * a dependency that names a marketplace met by the copy installed from it and enabled (its range
 * checked against that copy), or by an enabled claude.ai plugin of the same name (its range checked
 * against its version; one that states none passes). For any other plugin a claude.ai copy meets
 * nothing, and the `is not installed` message says so (only while that plugin is enabled: a disabled
 * one is not loaded, troubleshooting.md "Dependency errors"). Each issue carries its `errorType`.
 */
export const checkDependency = (
  dep: NormalizedDependency,
  found: DependencyCopies,
  loadsFromClaudeAi: boolean,
): DependencyIssue | undefined => call('checkDependency', dep, found, loadsFromClaudeAi);
