/**
 * Path arithmetic on the kit's one path form: absolute, `/`-separated, no `.` or `..` segments.
 * Pure string work, because the kit never calls the host's `path` module (docs/ARCHITECTURE.md, L3)
 * and path rules match on POSIX form everywhere (permissions.md, "Where a pattern anchors").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/path/posix.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PathContext } from '../types.ts';

const call = coreModule('rules/path/posix');

export type Platform = PathContext['platform'];

/**
 * A host path in POSIX form, `~` expanded and resolved against the session's current directory:
 * the one reading of a path the rules, the working directories and the modes share.
 */
export const absolutePath = (
  path: string,
  paths: Pick<PathContext, 'cwd' | 'home' | 'platform'>,
): string => call('absolutePath', path, paths);
