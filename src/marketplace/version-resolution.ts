import type { PluginSourceKind } from '../format/marketplace/types.ts';
import type { Sha256 } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The version of an installed plugin as Claude Code computes it (plugins/loading.md, "How Claude
 * Code computes the version"): for every source type except `command` the manifest's `version`,
 * then the entry's, then what the source gives; for a `command` source always what the command
 * produced. The version names the cache directory and decides whether an update changes anything,
 * so it is a decision the host reads, never recomputes. Pure over what the host reports; only the
 * hash of a `git-subdir` path goes through the `Sha256` port.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/version_resolution.rs`,
 * docs/RUST.md).
 */
const call = coreModule('marketplace/version-resolution');

/** The version of a source that gives nothing to version it by (loading.md: "`unknown`"). */
export const UNKNOWN_VERSION: 'unknown' = call('UNKNOWN_VERSION');

/** Where a version came from; `claude-ai` is the version a marketplace hosted on claude.ai records. */
export type VersionOrigin =
  | 'manifest'
  | 'entry'
  | 'sha'
  | 'sha256'
  | 'command-hash'
  | 'unknown'
  | 'claude-ai';

export interface PluginVersion {
  readonly version: string;
  readonly origin: VersionOrigin;
}

/** The source of a plugin: a `git-subdir` source always has its `path`, which its version hashes. */
export type VersionSource =
  | { readonly sourceKind: Exclude<PluginSourceKind, 'git-subdir'>; readonly subdir?: undefined }
  | {
      readonly sourceKind: 'git-subdir';
      /** `path` of the `git-subdir` source. */
      readonly subdir: string;
    };

/** What the host knows about a plugin when the version is decided; every fact it lacks is left out. */
interface VersionFacts {
  /** `version` in the plugin's manifest (the merged manifest when the entry supplies it, `mergeEntryWithManifest`). */
  readonly manifestVersion?: string;
  readonly entryVersion?: string;
  /** The commit of the cloned repository or, for a relative path, of the installed directory. */
  readonly commitSha?: string;
  /** The `sha256` pin of an `archive` entry. */
  readonly archiveSha256?: string;
  /** The digest of the downloaded archive, used when the entry has no pin. */
  readonly archiveDigest?: string;
  /** The hash of what a `command` source produced (`copyModeHash`, `linkModeHash`). */
  readonly commandHash?: string;
  /**
   * A relative-path plugin whose directory or whose marketplace is a Git repository: either one
   * gives a commit to version it by; neither gives `unknown` ("Local directory, when neither the
   * plugin directory nor its marketplace is a git repository").
   */
  readonly inGitRepository: boolean;
  /** The only repository the host found encloses the install path (a git-managed `~/.claude`): its commit is not a version. */
  readonly enclosingRepoIgnored: boolean;
  /** The version a marketplace hosted on claude.ai records for the plugin. */
  readonly recordedByClaudeAi?: string;
}

export type VersionInput = VersionFacts & VersionSource;

/**
 * Whether a version can name one folder: "The version also names the plugin's cache directory"
 * (loading.md), so a version that is empty, `.`, `..` or holds a separator or a NUL would leave
 * `cache/<marketplace>/<plugin>/`. The docs do not restrict the characters; any other text is kept
 * verbatim (spec/judgments.md).
 */
export const isVersionSegment = (version: string): boolean => call('isVersionSegment', version);

/** A stated version that also names one folder; one that does not is skipped, as if unset (spec/judgments.md). */
export const usableVersion = (value: string | undefined): string | undefined =>
  call('usableVersion', value);

/**
 * The version of a plugin. A plugin of a marketplace hosted on claude.ai has "the version claude.ai
 * records for the plugin ... and the manifest's `version` isn't read".
 */
export const resolvePluginVersion = (input: VersionInput, sha256: Sha256): Promise<PluginVersion> =>
  call.async('resolvePluginVersion', input, sha256);
