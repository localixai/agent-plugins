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
 * `C:\Users\alice` becomes `/c/Users/alice`: "On Windows, paths are normalized to POSIX form before
 * matching" (permissions.md). A UNC path `\\server\share` keeps its two leading slashes' meaning
 * through the later folding to `/server/share`; networks are told apart on the raw spelling
 * (`network.ts`).
 */
export const toPosixPath = (path: string, platform: Platform): string =>
  call('toPosixPath', path, platform);

/** Folds `.`, `..` and repeated slashes of a POSIX path; the result is absolute. */
export const normalizeAbsolute = (path: string): string => call('normalizeAbsolute', path);

/** `path` against `base` (a normalised absolute path), normalised; an absolute `path` ignores `base`. */
export const resolvePath = (base: string, path: string): string => call('resolvePath', base, path);

/**
 * A host path in POSIX form, `~` expanded and resolved against the session's current directory:
 * the one reading of a path the rules, the working directories and the modes share.
 */
export const absolutePath = (
  path: string,
  paths: Pick<PathContext, 'cwd' | 'home' | 'platform'>,
): string => call('absolutePath', path, paths);

/**
 * The spelling two paths are compared in. Windows names compare without case and drop the dots and
 * spaces that end them (`.GIT`, `.git.` and `.git ` are `.git`); POSIX names compare as written,
 * macOS included (spec/judgments.md).
 */
export const foldPath = (path: string, platform: Platform): string =>
  call('foldPath', path, platform);

/**
 * The text of `path` below `root` (`''` for the root itself), or `undefined` when it is not below;
 * on Windows both are folded first (`foldPath`), and so is the text returned.
 */
export const relativeWithin = (
  root: string,
  path: string,
  platform: Platform = 'posix',
): string | undefined => call('relativeWithin', root, path, platform);

export const dirnameOf = (path: string): string => call('dirnameOf', path);

/** Whether a segment holds a gitignore wildcard (or an escape), so a literal prefix ends before it. */
export const hasGlobSyntax = (segment: string): boolean => call('hasGlobSyntax', segment);
