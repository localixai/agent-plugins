/**
 * The checks made before anything is fetched (plugins/marketplace-reference.md, "url plugin
 * source", "git-subdir plugin source", "npm plugin source", "archive plugin source"): pure, they
 * return issues, and a refusal "names the value and the reason". The docs give the rules and, for
 * npm, the categories; the wording of each message is the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/source_safety.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { GitRepository } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/source-safety');

export interface GitReading {
  readonly repo: GitRepository | undefined;
  readonly partialClone: boolean;
  readonly issues: readonly PluginIssue[];
}

/**
 * A git address in a `url` or `git-subdir` plugin source, or a `git` marketplace source. `url`
 * "doesn't take `owner/repo` shorthand", nor does a marketplace's `git`; `git-subdir` accepts it
 * as GitHub shorthand.
 */
export const readGitAddress = (text: string, shorthand: boolean, path: string): GitReading =>
  call('readGitAddress', text, shorthand, path);

/** GitHub naming rules for an owner and a repository ("Owner and repo must follow GitHub naming rules"). */
export const isGitHubRepository = (owner: string, repo: string): boolean =>
  call('isGitHubRepository', owner, repo);

/** `github` takes `owner/repo`: one rule for a plugin source, a marketplace source and `marketplace add`. */
export const githubRepoIssues = (repo: string, path: string): PluginIssue[] =>
  call('githubRepoIssues', repo, path);

/**
 * A path inside a cloned repository stays there: a `git-subdir` path (the docs say only "the
 * subdirectory that holds the plugin"), and a marketplace's `path` and each of its `sparsePaths`.
 */
export const repositoryPathIssues = (inside: string, path: string): PluginIssue[] =>
  call('repositoryPathIssues', inside, path);

/** `ref` is "a branch or tag": one rule for the plugin and the marketplace git sources. */
export const refIssues = (ref: string | undefined, path: string): PluginIssue[] =>
  call('refIssues', ref, path);

// ---- npm ----

export interface NpmSafety {
  /** The registry URL of the installing user's own npm configuration. */
  readonly defaultRegistry?: string;
}

/** What an npm source names: a tarball link, or a package name and the version npm fetches. */
export type NpmSpec =
  | { readonly tarballUrl: string }
  | { readonly package: string; readonly version: string };

/**
 * `package` as npm reads it: a tarball link, or a name with a version appended after the first `@`
 * past a scope (it wins over `version`), or a bare name whose version is `version`, else `latest`.
 * A package name holds no other `@`, so `x@git+ssh://git@host/x.git` appends `git+ssh://git@host/x.git`.
 */
export const npmSpecOf = (pkg: string, version: string | undefined): NpmSpec =>
  call('npmSpecOf', pkg, version);

interface NpmPackageSource {
  readonly package: string;
  readonly version?: string;
}

/**
 * "Claude Code checks the `package` value before fetching anything." The kinds of value and what
 * each means are the registry-only rule of `node-deps-spec`, shared with the dependency install;
 * the same rule judges the version a name carries or the `version` field gives, so `name@<git
 * address>` is refused as the git address is. `path` is the source's (`source`). The fields cross
 * alone: a caller passes its whole source and its whole safety.
 */
export const npmPackageIssues = (
  source: NpmPackageSource,
  safety: NpmSafety,
  path: string,
): PluginIssue[] =>
  call(
    'npmPackageIssues',
    { package: source.package, version: source.version },
    { defaultRegistry: safety.defaultRegistry },
    path,
  );

/** The `registry` URL "must use `https` unless it is the installing user's own default npm registry". */
export const npmRegistryIssues = (
  registry: string,
  safety: NpmSafety,
  path: string,
): PluginIssue[] =>
  call('npmRegistryIssues', registry, { defaultRegistry: safety.defaultRegistry }, path);

// ---- archive ----

/** `url` "must use `https://` and can't point at a loopback, link-local, or cloud-metadata host". */
export const archiveUrlIssues = (url: string, path: string): PluginIssue[] =>
  call('archiveUrlIssues', url, path);
