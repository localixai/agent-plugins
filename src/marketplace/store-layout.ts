/**
 * Where Claude Code keeps plugin files and state records (plugins/loading.md, "Find plugins on
 * disk"; env-vars.md, `CLAUDE_CODE_PLUGIN_CACHE_DIR`): one plugins root, and under it the version
 * cache, the persistent data folders, the marketplace clones, the claude.ai sync folders, the JSON
 * records and the dated copies Claude Code keeps before it rewrites `installed_plugins.json`.
 *
 * Pure path builders over a host-supplied `join` (the host's own separator, as `Locations.joinPath`):
 * the kit never concatenates a path with its own separator and uses no `node:path` (L3).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/store_layout.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/store-layout');

/** `base` and a `/`-separated relative path joined the way the host writes paths (`Locations.joinPath`). */
export type PathJoin = (base: string, relativePath: string) => string;

/**
 * The plugins root: `CLAUDE_CODE_PLUGIN_CACHE_DIR` when set, else `plugins` under the configuration
 * directory (`CLAUDE_CONFIG_DIR`: "All settings, session history, and plugins are stored under this
 * path"), else `~/.claude/plugins`. "Despite the name, this sets the parent directory, not the cache
 * itself" (env-vars.md). Judgments (spec/judgments.md): the docs do not say which wins when both
 * variables are set, so the one that names the plugins root itself wins over the configuration
 * directory; an empty value is no override, as for an unset variable.
 */
export const resolvePluginsRoot = (options: {
  readonly home: string;
  readonly override?: string;
  /** `CLAUDE_CONFIG_DIR`: the directory every `~/.claude` path lives under when it is set. */
  readonly configDir?: string;
  readonly join: PathJoin;
}): string => call('resolvePluginsRoot', options);

/** The names the table of "Find plugins on disk" gives; each is directly under the root. */
export const STORE_NAMES: {
  readonly cache: 'cache';
  readonly data: 'data';
  readonly marketplaces: 'marketplaces';
  readonly synced: 'synced';
  readonly trash: '.trash';
  readonly installedPlugins: 'installed_plugins.json';
  readonly knownMarketplaces: 'known_marketplaces.json';
  readonly knownMarketplacesClaudeAi: 'known_marketplaces_claudeai.json';
  readonly flaggedPlugins: 'flagged-plugins.json';
} = call('STORE_NAMES');

export interface StoreLayout {
  readonly root: string;
  /** A file or folder directly under the root, by name (a name listed in `STORE_NAMES` or a dated copy). */
  file(name: string): string;
  cache(): string;
  /** `cache/<marketplace>/<plugin>/<version>/`: `${CLAUDE_PLUGIN_ROOT}` of a copied marketplace plugin. */
  versionDir(marketplace: string, plugin: string, version: string): string;
  /** `data/<plugin-id>/`; `dataId` is the sanitized `<plugin-id>` (`pluginDataId`, `loading/resolve`). */
  pluginData(dataId: string): string;
  /** `marketplaces/<name>/`: the clone or download of a GitHub, git or URL marketplace. */
  marketplace(name: string): string;
  synced(): string;
  trash(): string;
  installedPlugins(): string;
  knownMarketplaces(): string;
  /** A marketplace hosted on claude.ai is recorded here instead of in `known_marketplaces.json`. */
  knownMarketplacesClaudeAi(): string;
  flaggedPlugins(): string;
  /** The copy of the install records dropped for names no version can use. */
  setAsideCopy(date: string, hash: string): string;
  /** The copy of an `installed_plugins.json` that could not be read. */
  unreadableCopy(date: string, hash: string): string;
}

/**
 * The layout under `root`. Judgment (spec/judgments.md): `<date>` and `<hash>` of the dated copies
 * are the caller's text (the docs give the shape, not the date format or the hash input).
 * The core's handle has methods only, so `root` is set on the object it becomes.
 */
export const createStoreLayout = (root: string, join: PathJoin): StoreLayout => {
  const methods: Omit<StoreLayout, 'root'> = call('createStoreLayout', root, join);
  return Object.assign(methods, { root });
};

/** What a name directly under the plugins root is, as the table of "Find plugins on disk" lists them. */
export type StoreEntry =
  | 'cache'
  | 'data'
  | 'marketplaces'
  | 'synced'
  | 'trash'
  | 'installed-plugins'
  | 'known-marketplaces'
  | 'known-marketplaces-claudeai'
  | 'flagged-plugins'
  | 'set-aside-copy'
  | 'unreadable-copy'
  | 'unknown';

export const classifyStoreEntry = (name: string): StoreEntry => call('classifyStoreEntry', name);
