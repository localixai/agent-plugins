import type { PluginIssue } from '../format/issues.ts';
import type { ScanFolder } from '../ports/folder-scan.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The walk of a scope folder, shared by the skills and the agents scans: the entries of a directory
 * with symbolic links followed, and the files of a tree. A link to a directory elsewhere on disk is
 * followed (skills.md, "Choose where skills load", "Symlinked folders"); one that dangles is skipped
 * with a warning, and a tree is walked at most `MAX_DEPTH` levels deep and each real directory once,
 * so a link loop ends (judgment, spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/foundation/folder_walk.rs`, docs/RUST.md).
 */

const call = coreModule('foundation/folder-walk');

export const MAX_DEPTH: number = call('MAX_DEPTH');

export interface Child {
  readonly name: string;
  /** Relative to the folder opened. */
  readonly path: string;
  readonly isFile: boolean;
  readonly isDirectory: boolean;
  /** Where the entry really is; absent only when the host could not say. */
  readonly canonical?: string;
}

export interface Walked<T> {
  readonly found: readonly T[];
  readonly issues: readonly PluginIssue[];
}

/** The entries of `directory`, sorted by name, each classified through its link. */
export const listChildren = (
  folder: ScanFolder,
  directory: string,
  signal?: AbortSignal,
): Promise<Walked<Child>> => call.async('listChildren', folder, directory, signal);

export interface WalkedFile {
  /** Relative to the folder opened. */
  readonly path: string;
  /** The names below the walked directory: `frontend`, `component.md`. */
  readonly segments: readonly string[];
  readonly canonical?: string;
}

/** The files below `root` with the extension (`.md`, case ignored), at any depth, sorted by path. */
export const walkFiles = (
  folder: ScanFolder,
  root: string,
  options: { readonly extension: string; readonly signal?: AbortSignal },
): Promise<Walked<WalkedFile>> => call.async('walkFiles', folder, root, options);
