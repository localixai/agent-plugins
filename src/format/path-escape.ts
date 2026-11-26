import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { PathPlatform } from './path-text.ts';

/**
 * A component path that leaves the plugin (plugins/loading.md, "Paths that escape the plugin
 * directory"): "Whether a plugin loads in place or from a cached copy, Claude Code doesn't let it
 * declare components outside its own directory", whether the path is declared in `plugin.json` or
 * in a marketplace entry. Rejected: a path that points outside as written, a symlink that leads
 * outside the plugin "other than links between plugins within one marketplace", and on macOS and
 * Linux a path with a backslash. The plugin "loads without that component". The loader (`locate`)
 * decides every declared path through `escapedPathIssue`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/path_escape.rs`, docs/RUST.md).
 */
const call = coreModule('format/path-escape');

/** The real paths (`/`-separated, one spelling) a link is judged against. */
export interface LinkRoots {
  readonly pluginRoot: string;
  /** The marketplace the plugin is in, when it is in one; links to other plugins of it are allowed. */
  readonly marketplaceRoot?: string;
}

export interface DeclaredPath {
  /** The field that declared the path: `commands`, `hooks`... */
  readonly component: string;
  /** The path as written. */
  readonly written: string;
  readonly platform: PathPlatform;
  /**
   * Where the path really leads when it is, or crosses, a symlink: an absolute path, or one
   * relative to the plugin folder; absent when it is not.
   */
  readonly resolvesTo?: string;
  /**
   * The folders to judge a link by. Without them nothing but the plugin's own folder is known, so
   * a link target that is absolute or climbs out of the folder leaves the plugin (judgment).
   */
  readonly roots?: LinkRoots;
  /**
   * The path is one of the plugin's own files (`hooks/hooks.json`, `monitors/monitors.json`,
   * `settings.json`, `SKILL.md` at the root): errors.md, "Path escapes plugin directory", reports it
   * by its full path, and in a local marketplace says it is outside that marketplace too.
   */
  readonly own?: boolean;
}

/** The issue that drops the path, or `undefined` when it stays inside the plugin. */
export const escapedPathIssue = (path: DeclaredPath): PluginIssue | undefined =>
  call('escapedPathIssue', path);
