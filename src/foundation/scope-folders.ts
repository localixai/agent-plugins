import type { PluginIssue } from '../format/issues.ts';
import type { FolderScan, ScanFolder } from '../ports/folder-scan.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The scope folders of a session, shared by the skills, the agents and the saved-workflows scans:
 * where each scope's base directories are (the managed directory, the project and its parents, the
 * added directories, the home), and the files of one folder below a base, read through the host's
 * `FolderScan` port. Each scan keeps its own table of scopes and its own parser; this is what the
 * tables and the parsers stand on.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/foundation/scope_folders.rs`, docs/RUST.md).
 */

const call = coreModule('foundation/scope-folders');

/** What a scan of the scope folders is asked for. */
export interface ScopeRequest {
  readonly home: string;
  /** The working directory the session started in (or moved to with `/cd`). */
  readonly cwd: string;
  /** The directories passed with `--add-dir` or `/add-dir`, in the order they were added. */
  readonly addedDirectories?: readonly string[];
  /**
   * `CLAUDE_CONFIG_DIR` as the session's environment gives it; unset or empty, `<home>/.claude`. The
   * user scope's documented folder is that directory (`format/config-directory`); the host's extra
   * scope folders of the user scope stay below `home`.
   */
  readonly configDir?: string | undefined;
}

export interface ScopeContext<R extends ScopeRequest = ScopeRequest> {
  readonly folders: FolderScan;
  readonly request: R;
  readonly signal?: AbortSignal;
  /**
   * Folders beside `.claude` that the host also reads skills, commands and agents from, below every
   * base of every scope that reads its `.claude` folder (`.agents`, `.localix`).
   */
  readonly extraScopeFolders?: readonly string[];
}

/** Where the base directories of a scope are; the user scope's are `SCOPE_PLACES.user`. */
export type ScopeBase = 'managed' | 'project' | 'repository' | 'added';

/** The base directories of each kind, in precedence order within it. */
export const SCOPE_BASES: {
  readonly [B in ScopeBase]: (context: ScopeContext) => Promise<readonly string[]>;
} = call('SCOPE_BASES');

/** One folder a scope reads its containers (`skills`, `agents`) below. */
export interface ScopePlace {
  /** The directory opened through the `FolderScan` port. */
  readonly base: string;
  /** The folder below the base: `.claude`, an extra scope folder, or `''` for the configuration directory itself. */
  readonly root: string;
  /** Its base's place among the bases of the scope, in precedence order. */
  readonly index: number;
  /** The documented folder (`.claude`, or the configuration directory), not one of the host's extra ones. */
  readonly documented: boolean;
}

/** The places of a scope for the scope folders asked, in precedence order. */
export type ScopePlaces<C extends ScopeContext = ScopeContext> = (
  context: C,
  roots: readonly string[],
) => Promise<readonly ScopePlace[]>;

/** The places of each kind of scope base, and of the user scope. */
export const SCOPE_PLACES: { readonly [B in ScopeBase | 'user']: ScopePlaces } =
  call('SCOPE_PLACES');

/** One file of a scope folder, with its text. */
export interface ScopeFile {
  /** Relative to the base opened. */
  readonly path: string;
  /** The path as the host names it, for messages and records. */
  readonly hostPath: string;
  /** The names below the folder: `frontend`, `component.md`. */
  readonly segments: readonly string[];
  /** Where the file really lives, when the host could say. */
  readonly canonical?: string;
  readonly text: string;
}

export interface ScopeFiles {
  readonly files: readonly ScopeFile[];
  /** A link that leads nowhere, a file the port refused. */
  readonly issues: readonly PluginIssue[];
}

export interface ScopeFilesOptions {
  /** The file extension, case ignored: `.md`. */
  readonly extension: string;
  /** `tree`: every depth ("recursively, so you can organize definitions into subfolders"); `flat`: the folder alone. */
  readonly depth: 'tree' | 'flat';
  readonly signal?: AbortSignal | undefined;
}

/**
 * The files of one folder below an opened base, with their text. A file the port refuses is skipped
 * with a warning (`readTextOf`); an aborted signal still rejects.
 */
export const readScopeFiles = (
  base: ScanFolder,
  container: string,
  options: ScopeFilesOptions,
): Promise<ScopeFiles> => call.async('readScopeFiles', base, container, options);

/** The folder to watch below an opened base: its host path when the scope watches it and it exists. */
export const watchedFolder = (
  base: ScanFolder,
  container: string,
  options: { readonly watched: boolean; readonly signal?: AbortSignal | undefined },
): Promise<readonly string[]> => call.async('watchedFolder', base, container, options);
