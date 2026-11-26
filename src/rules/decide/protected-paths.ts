import type { PathContext } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ProtectedPathOptions } from './context.ts';

/**
 * permission-modes.md, "Protected paths": "Protected directories" and "Protected files". A
 * directory matches wherever it sits in a path (`.git` of the project and of a nested folder);
 * `.config/git` is two segments; a file matches by its name. The lists are the docs' own.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/protected_paths.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/decide/protected-paths');

/**
 * Whether `path` is the target of a symlinked settings file (`options.symlinkTargets`) and is not
 * protected for another reason: "If the file's own path is also that of a settings file ... the write
 * goes to the classifier as other protected-path writes do".
 */
export const isSymlinkTargetOnly = (
  path: string,
  paths: PathContext,
  options: ProtectedPathOptions,
): boolean => call('isSymlinkTargetOnly', path, paths, options);

/**
 * Whether a write to `path` (absolute, POSIX) is a write to a protected path: a protected
 * directory or file, `.claude` apart from its exceptions, a file the session was started with
 * (`--settings`, `--mcp-config`, `--agents`), a directory loaded with `--plugin-dir`, or `dev-mods`.
 * The target of a symlinked settings file is `isSymlinkTargetOnly`.
 */
export const isProtectedPath = (
  path: string,
  paths: PathContext,
  options: ProtectedPathOptions = {},
): boolean => call('isProtectedPath', path, paths, options);
