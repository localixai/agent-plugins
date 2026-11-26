import type { PathContext } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * permission-modes.md, "Critical paths": an `rm` or `rmdir` whose target Claude Code "recognizes as a
 * critical path": the filesystem root, a top-level directory, the home directory, the working
 * directory and its parents, and (only as a glob under them) the additional working directories and
 * their parents. The table "Other targets that count as critical paths" adds the shell-variable
 * forms and command substitution. The docs say the check "can miss some commands": what it does not
 * recognize gets no special handling here either.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/critical_removal.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/decide/critical-removal');

/** The filesystem root, a top-level directory (a drive root or its children on Windows) or the home directory. */
export const isSystemDirectory = (directory: string, paths: PathContext): boolean =>
  call('isSystemDirectory', directory, paths);

/** Whether `directory` is critical: a system directory, the working directory or a parent of it. */
export const isCriticalDirectory = (directory: string, paths: PathContext): boolean =>
  call('isCriticalDirectory', directory, paths);

/** "Dangerous rm operation not ruled out": a command the kit cannot read to the end may hide a removal. */
export const cannotRuleOutRemoval = (command: string): boolean =>
  call('cannotRuleOutRemoval', command);

/**
 * Whether a shell command contains a recognized critical-path removal, anywhere in it: in a
 * compound command, a subshell, a command substitution or an inline `sh -c` script.
 */
export const removesCriticalPath = (
  command: string,
  paths: PathContext,
  depth = 0,
  bound = false,
): boolean => call('removesCriticalPath', command, paths, depth, bound);
