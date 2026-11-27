/**
 * What copying a plugin tree into the cache does with each entry (plugins/host-marketplace.md,
 * "Share files within a marketplace with symlinks"; plugins/marketplace-reference.md, "Output that
 * fails the install or update"): a plan over the entries a host listed without following links,
 * and over the real paths it resolved for each link. Nothing is read or written here.
 *
 * A link is handled "by where the target resolves": within the plugin it is kept as a relative
 * link, elsewhere in the marketplace it is dereferenced (the target's content is copied in its
 * place), outside the marketplace it is skipped. "For plugins installed from a local path, or from
 * a `command` source whose `mode` is the default `copy`", which have no marketplace to
 * dereference into, only a link within the plugin's own directory is kept.
 *
 * A dereferenced directory is the host's to list and plan again, with its children's paths below
 * the link's and its real path added to `TreeRoots.copying`. A link to a directory that is, or
 * contains, one being copied (the marketplace root, the plugin's own parent, a sibling that links
 * back) would copy the same tree again without end, so it is skipped as a `cycle` (spec/
 * judgments.md). A link whose target does not exist is skipped (judgment).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/copy_plan.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/copy-plan');

export interface TreeEntry {
  /** Where the entry lands, relative to the plugin root and `/`-separated. */
  readonly path: string;
  readonly isDirectory: boolean;
  readonly isSymlink: boolean;
  /** The size of a file; 0 for a directory or a link. */
  readonly bytes: number;
  /** For a link: the real path it finally resolves to; absent when it points at nothing. */
  readonly target?: { readonly path: string; readonly isDirectory: boolean };
}

export interface TreeRoots {
  /** The real path of the plugin directory, spelled as the targets are. */
  readonly pluginRoot: string;
  /** The marketplace's real path; absent for a plugin installed from a local path or in copy mode. */
  readonly marketplaceRoot?: string;
  /**
   * The real paths of the directories already being copied above the one planned now: each
   * dereferenced directory the host went into, in order. The plugin root is always one.
   */
  readonly copying?: readonly string[];
}

export interface Relink {
  readonly path: string;
  /** The link's new target, relative to the folder the link is in and `/`-separated. */
  readonly target: string;
}

export interface Dereference {
  readonly path: string;
  /** The real path whose content is copied where the link was. */
  readonly from: string;
  readonly isDirectory: boolean;
}

export type SkipReason = 'outside-marketplace' | 'dangling' | 'cycle';

/** The size limits of a tree copied whole: at most `maxBytes` of files and `maxEntries` entries. */
export interface CopyLimits {
  readonly maxBytes: number;
  readonly maxEntries: number;
}

export interface TreePlan {
  /** Files and directories copied as they are. */
  readonly copy: readonly string[];
  readonly relinks: readonly Relink[];
  readonly dereferences: readonly Dereference[];
  readonly skipped: readonly { readonly path: string; readonly reason: SkipReason }[];
  /** The copy cannot be made: it is too large. */
  readonly refusals: readonly PluginIssue[];
}

/** A plan of copying `entries`; `limits` apply to a tree that is copied whole (a `command` source in copy mode). */
export const planTreeCopy = (
  entries: readonly TreeEntry[],
  roots: TreeRoots,
  limits?: CopyLimits,
): TreePlan => call('planTreeCopy', entries, roots, limits);

/**
 * The refusal for a plugin directory of a fetched marketplace that is, or passes through, a link
 * that resolves outside the marketplace ("an entry of a fetched marketplace that resolves or opens
 * outside its tree", plugins/troubleshooting.md): the listing line `claude plugin list` shows.
 */
export const pluginRootIssue = (input: {
  readonly entryPath: string;
  readonly pluginRoot: string;
  readonly marketplaceRoot: string;
}): PluginIssue | undefined => call('pluginRootIssue', input);
