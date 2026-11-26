import type { PluginIssue } from '../format/issues.ts';
import type { PluginFiles } from '../ports/files.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * A Git LFS pointer file, which is what an LFS-tracked file is after a clone (plugins/host-
 * marketplace.md, "Keep plugin files out of Git LFS": "The clone never downloads LFS content, so
 * LFS-tracked files arrive as pointer files"). The docs do not describe the pointer's text; the
 * layout is Git LFS's own (a `version` line naming its spec, an `oid sha256:` line, a `size` line),
 * a judgment (spec/judgments.md). A pointer is a small text file, so a host reads at most
 * `MAX_POINTER_BYTES` of a file to ask. Git writes the lines with `\n`, and a checkout that converts
 * line ends leaves `\r\n`; both are read (judgment).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/lfs_pointer.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/lfs-pointer');

export interface LfsPointer {
  /** The SHA-256 of the real content, lower case hex. */
  readonly oid: string;
  /** The size of the real content in bytes. */
  readonly size: number;
}

/** The pointer a text is, or `undefined` for any other text. */
export const readLfsPointer = (text: string): LfsPointer | undefined =>
  call('readLfsPointer', text);

/** The warning for a plugin file that is a pointer: the plugin needs the real file and cannot get it. */
export const lfsPointerIssue = (path: string): PluginIssue => call('lfsPointerIssue', path);

/**
 * The install check for a git-based plugin that was cloned: one warning per file of the plugin that
 * is a Git LFS pointer, since the clone never downloads LFS content and the plugin needs the real
 * file. Lists the plugin through its files (not into `.git`, not through links) and reads only
 * files small enough to be a pointer. Ordered by path.
 */
export const lfsPointerIssues = (
  files: PluginFiles,
  options: { readonly signal?: AbortSignal | undefined } = {},
): Promise<readonly PluginIssue[]> => call.async('lfsPointerIssues', files, options);
