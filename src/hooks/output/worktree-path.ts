/**
 * What a `WorktreeCreate` hook returned, read as the worktree path (hooks.md, "WorktreeCreate
 * output"): a command hook prints it "as the last non-empty line of stdout" and "Claude Code strips
 * ANSI escape codes before reading that line"; an HTTP hook returns `hookSpecificOutput.worktreePath`.
 * "If the hook fails or produces no path, worktree creation fails." A relative path is resolved
 * "against the directory the hook ran in, collapsing any `.` or `..` segments in it"; "Claude Code
 * refuses an absolute path that contains `.` or `..` segments, and any path that passes through a
 * symlink below the repository root", and "the error names the rejected component". Whether the
 * resulting path is a directory the session can enter is the host's: it is the one that can look.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/worktree_path.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/worktree-path');

/** `text` without its ANSI escape sequences (CSI, OSC and two-character escapes). */
export const stripAnsi = (text: string): string => call('stripAnsi', text);

/** The last non-empty line of `stdout` after the ANSI escapes are stripped, or `undefined`. */
export const lastPathLine = (stdout: string): string | undefined => call('lastPathLine', stdout);

/** The path, or why creation fails. */
export type WorktreePath = { readonly path: string } | { readonly failure: string };

/** What the host knows about where the hook ran. */
export interface WorktreeLocation {
  /** The directory the hook ran in: a relative path is resolved against it. */
  readonly ranIn: string;
  /** The repository root: only a symlink below it is refused. */
  readonly repositoryRoot?: string;
  /**
   * Whether a path is itself a symbolic link. The kit reads no file system, so the host answers;
   * without it no path is checked for links.
   */
  readonly isSymlink?: (path: string) => Promise<boolean>;
}

/** What a merged `WorktreeCreate` outcome holds of the hooks' answers. */
export interface WorktreeAnswer {
  readonly proceed: boolean;
  /** The stdout of each command hook that exited 0. */
  readonly stdout: readonly string[];
  readonly effects: { readonly worktreePath?: string };
}

/**
 * The worktree path a `WorktreeCreate` outcome asks the session to use, or why creation fails.
 *
 * Judgment (spec/judgments.md): the docs name the refusals and say the error names the rejected
 * component, not the words.
 */
export const resolveWorktreePath = (
  answer: WorktreeAnswer,
  location: WorktreeLocation,
): Promise<WorktreePath> => call.async('resolveWorktreePath', answer, location);
