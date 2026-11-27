/**
 * Path text: the one owner of what a path says as written (absolute, climbing, where it resolves)
 * and of the `PluginFiles` spelling (plugins/loading.md, "Paths that escape the plugin
 * directory"; plugins/manifest-reference.md, "Path rules"). Pure over strings: nothing is looked
 * up. A backslash is a separator on Windows only; "on macOS and Linux" it is part of a name.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/path_text.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

/** Whose path rules apply: a backslash separates on `windows` only. */
export type PathPlatform = 'posix' | 'windows';

const call = coreModule('format/path-text');

/** A path from a file system root: `/x`, `\x`, or a drive (`C:`, `C:/x`, `C:\x`). */
export const isAbsolutePath = (path: string): boolean => call('isAbsolutePath', path);

/**
 * The named segments of a relative path once `.`, empty segments and `..` are resolved, or
 * `undefined` when it climbs above where it starts.
 */
export const resolveSegments = (path: string, platform: PathPlatform): string[] | undefined =>
  call('resolveSegments', path, platform);

/** Whether a path names a place outside the folder it is relative to: absolute, or climbing above it. */
export const leavesFolder = (path: string, platform: PathPlatform): boolean =>
  call('leavesFolder', path, platform);

/**
 * Whether a component path leaves the plugin as written (plugins/loading.md, "Paths that escape the
 * plugin directory"): it climbs with `..`, is absolute, or, "on macOS and Linux, contains a
 * backslash anywhere in it, even when the path stays inside the plugin". On Windows a backslash is
 * a separator, so `..\x` climbs there too. Any `..` segment counts, even one that comes back down.
 */
export const escapesAsWritten = (path: string, platform: PathPlatform): boolean =>
  call('escapesAsWritten', path, platform);

/**
 * A component path as a `PluginFiles` path: relative to the root, `/`-separated, no `.` or empty
 * segment, no trailing slash; the plugin root is `.` ("Both `"."` and `"./"` denote the plugin
 * root"). `.//x` is `x`, as a file system resolves it under the root: the result never starts
 * with `/`, so a port that resolves it against the root stays there. On Windows a backslash
 * separates like `/`. `..` is kept; `locate` rejects it before any read.
 */
export const rootRelative = (path: string, platform: PathPlatform = 'posix'): string =>
  call('rootRelative', path, platform);

/**
 * Where `path` leads from the folder `root` (an absolute, `/`-spelled path): an absolute `path`
 * as itself in `/` spelling, a relative one joined to `root` with `.` and `..` resolved.
 */
export const resolveFrom = (root: string, path: string, platform: PathPlatform): string =>
  call('resolveFrom', root, path, platform);

/** `path` with no trailing `/` (a root `/` becomes empty). */
export const withoutTrailingSlashes = (path: string): string =>
  call('withoutTrailingSlashes', path);

/** True when `path` is `root` or inside it, by whole path segments (both in one `/` spelling). */
export const isWithin = (root: string, path: string): boolean => call('isWithin', root, path);
