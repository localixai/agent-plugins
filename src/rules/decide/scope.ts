import { coreModule } from '../../wasm/bridge.ts';
import type { PathContext } from '../types.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/scope.rs`, docs/RUST.md). */
const call = coreModule('rules/decide/scope');

/** A host path in the kit's one form: POSIX, absolute, `.` and `..` folded. */
export const posixOf = (path: string, paths: PathContext): string => call('posixOf', path, paths);

/**
 * Whether `path` is the working directory or an additional directory, or below one. Each is taken
 * with its real location when it is reached through a link (`PathContext.realDirectories`): a file
 * the symlink check resolves below the real location sits in that working directory (permissions.md,
 * "Writes through a symlink"; a judgment, spec/judgments.md).
 */
export const inScope = (path: string, paths: PathContext): boolean => call('inScope', path, paths);
