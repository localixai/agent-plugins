import type { PluginIssue } from '../format/issues.ts';
import type { PluginSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { NormalizedDependency } from './types.ts';

/**
 * How a dependency's version constraint meets git tags (plugins/dependencies.md, "Release a plugin
 * that others depend on", "How a constraint resolves against tags", "Combine constraints from
 * several plugins"). A dependency with a range installs at the highest tag `<plugin-name>--v<version>`
 * that satisfies it; tags come from the `GitTags` port, so everything here is pure.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/dependency_tags.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/dependency-tags');

/** A release tag and the version it names. */
export interface ReleaseTag {
  readonly tag: string;
  readonly version: string;
}

/** `<plugin-name>--v<version>` (plugins/dependencies.md, "Create a release tag"). */
export const releaseTag = (name: string, version: string): string =>
  call('releaseTag', name, version);

/**
 * The version a tag names for `name`, or `undefined` for another plugin's tag or one whose version
 * is not strict semver. Judgment (spec/judgments.md): a `v`, `=` or blank before the version
 * (`name--vv2.1.0`) is not a release tag; the docs name only `<name>--v<version>`.
 */
export const parseReleaseTag = (tag: string, name: string): string | undefined =>
  call('parseReleaseTag', tag, name);

/**
 * The highest tag of `name` that satisfies `range`, with node-semver's rules: a range never matches
 * a pre-release unless it opts in (`^2.0.0-0`). `undefined` when none does.
 */
export const selectTag = (
  tags: readonly string[],
  name: string,
  range: string,
): ReleaseTag | undefined => call('selectTag', tags, name, range);

/**
 * The tag a dependency resolves to when several installed plugins constrain it: the highest that
 * satisfies every range. A `PluginIssue` when the ranges cannot all hold: `has an invalid version
 * requirement`, `has version requirements too complex to intersect` or `has conflicting version
 * requirements` (with the ranges listed); `undefined` when they overlap but no tag falls in the
 * overlap, which is when the marketplace's current copy is fetched (`autoUpdateDecision` decides
 * what that means). The dependency's `name` reads the tags; its id names it in a message. Judgment:
 * the text after the quoted words, and 64 combinations of `||` alternatives as the limit of "too complex".
 */
export const combineRanges = (
  ranges: readonly string[],
  tags: readonly string[],
  dep: NormalizedDependency,
): ReleaseTag | PluginIssue | undefined => call('combineRanges', ranges, tags, dep);

/** What auto-update does with a dependency that installed plugins constrain (plugins/dependencies.md, "Combine constraints from several plugins"). */
export type AutoUpdateDecision =
  | {
      readonly outcome: 'update';
      readonly via: 'tag';
      readonly tag: string;
      readonly version: string;
    }
  | { readonly outcome: 'update'; readonly via: 'current-copy'; readonly version?: string }
  | { readonly outcome: 'skip'; readonly reason: 'ranges-conflict'; readonly issue: PluginIssue }
  | {
      readonly outcome: 'skip';
      readonly reason: 'current-copy-out-of-range';
      readonly version?: string;
    };

export interface AutoUpdateInput {
  readonly dep: NormalizedDependency;
  /** The range of every installed plugin that constrains the dependency. */
  readonly ranges: readonly string[];
  readonly tags: readonly string[];
  /** The `version` in the plugin.json of the marketplace's current copy. */
  readonly currentCopyVersion?: string;
}

/**
 * Whether auto-update moves a constrained dependency, and to what (plugins/dependencies.md, "Combine
 * constraints from several plugins"): to the highest tag that satisfies every installed plugin's
 * range; when the ranges do not overlap, nowhere (the dependency stays at its current version and
 * the Errors tab names the constraining plugin: `issue` is that conflict); when they overlap and no
 * tag falls in the overlap, to the marketplace's current copy, unless its `version` falls outside any
 * range, which skips the update. A copy that states no version satisfies no range (judgment).
 */
export const autoUpdateDecision = (input: AutoUpdateInput): AutoUpdateDecision =>
  call('autoUpdateDecision', input);

/** What the constraint was checked against: a tag, or the marketplace's current copy (`version` is its `plugin.json` version, absent when it has none). */
export interface ResolvedVersion {
  readonly via: 'tag' | 'current-copy';
  readonly tag?: string;
  readonly version?: string;
}

export interface DependencyResolution {
  readonly dep: NormalizedDependency;
  /**
   * Every range that constrains the dependency when several installed plugins do (`dep.range` is
   * the one of the plugin being installed); absent, `dep.range` alone applies.
   */
  readonly ranges?: readonly string[];
  readonly source: PluginSource;
  /** The tags of the repository that hosts the plugin (empty for a source with none, or a folder that is not a git repository). */
  readonly tags: readonly string[];
  /** The `version` in the plugin.json of the marketplace's current copy. */
  readonly currentCopyVersion?: string;
}

/**
 * The version `claude plugin list` shows for a resolved dependency (plugins/dependencies.md,
 * "Confirm the resolved version"): a tag-resolved copy carries "a 12-character commit suffix, such as
 * `2.1.0-8713c5b11005`"; the commit is the one the tag names, which the host knows. A copy with no
 * commit known shows the bare tag version (judgment), a current copy its `plugin.json` version.
 */
export const resolvedVersionLabel = (
  resolved: ResolvedVersion,
  commit: string | undefined,
): string | undefined => call('resolvedVersionLabel', resolved, commit);

/**
 * What a dependency's constraint resolves against, by the kind of its plugin source: a tag of its
 * own repository (`github`, `url`, `git-subdir`: none in range is `has no git tag satisfying`), a
 * tag of the marketplace repository or else its current copy (`relative`), or the `plugin.json`
 * version of the fetched plugin (`npm`, `archive`, `command`). The `version` returned is the one a
 * constraint is checked against: the tag's, which beats `plugin.json`, when resolved by tag.
 */
export const resolveDependencyVersion = (
  input: DependencyResolution,
): ResolvedVersion | PluginIssue => call('resolveDependencyVersion', input);
