import type { FileStat, PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { LinkRoots } from './path-escape.ts';
import type { PathPlatform } from './path-text.ts';

const call = coreModule('format/locate');

/**
 * Containment and existence of a plugin's paths, and the context every loader of plugin paths
 * shares.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/locate.rs`, docs/RUST.md).
 */

/** What a path turned out to be: what it is, or the issue that stops it loading; neither when it is missing. */
export interface Located {
  readonly stat?: FileStat;
  readonly issue?: PluginIssue;
}

/** What `locate` needs: the folder, and what decides whether a path leaves it. */
export interface LocateContext {
  readonly files: PluginFiles;
  /** The backslash rule is the platform's: "on macOS and Linux" a backslash escapes. */
  readonly platform: PathPlatform;
  /** Where the plugin and its marketplace really are, so links between plugins of one marketplace load. */
  readonly roots?: LinkRoots;
  readonly signal?: AbortSignal;
}

/** What a caller may leave out of a `LocateContext`. */
export type LocateOptions = Partial<Omit<LocateContext, 'files'>>;

/**
 * The context every loader of plugin paths shares. `platform` defaults to `posix`, the stricter
 * one: a backslash in a declared path escapes there.
 */
export const locateContext = (files: PluginFiles, options: LocateOptions = {}): LocateContext =>
  call('locateContext', files, options);

/**
 * Containment and existence (plugins/manifest-reference.md, "Containment and existence"; loading.md,
 * "Paths that escape the plugin directory"): a path with `..`, an absolute path, a backslash on
 * macOS and Linux, or a symlink that leads outside the plugin (other than to another plugin of
 * its marketplace) "doesn't load" and is reported as `<component> path escapes plugin directory:
 * <path>` (`escapedPathIssue`); a path that is not there is `undefined` and the caller decides
 * whether it was declared. A path the system could not check (`FileStat.checkFailed`, on it or a
 * folder above it) does not load either: `<component> path could not be checked: <path> (<code>)`
 * (errors.md, "Path could not be checked"), for a default location as for a declared one, the
 * plugin's other components loading. `written` is the spelling the message quotes. A link is seen on the
 * path itself and on every folder above it: each ancestor is stat'd once per call, since a port
 * reports a link only on the entry it is asked about (`FileStat.realPath`). Every reader of a
 * declared or standard plugin path decides containment here, and nowhere else.
 */
export const locate = (
  context: LocateContext,
  component: string,
  written: string,
  own?: boolean,
): Promise<Located> => call.async('locate', context, component, written, own);

/**
 * The load error of a declared path that is not there, `<component> path not found: <absolute
 * path>` (spec §2.5 item 3): the plugin root joined with the path when the host gave `roots`, the
 * path as written when it did not (the loader then knows no host path).
 */
export const missingAt = (
  context: LocateContext,
  component: string,
  written: string,
): PluginIssue => call('missingAt', context, component, written);
