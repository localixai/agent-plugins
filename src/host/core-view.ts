import type { CorePackage } from '../loading/core/index.ts';
import type { Plugin } from '../loading/index.ts';
import type { HostCorePackage, HostPlugin } from './types.ts';

/**
 * A core package as the session's plugin machinery sees it (Agent Plugins Extended §18.1.5, J18.15):
 * a plugin to every part of the session that is not a component, known by its id, enabled,
 * restricted, named in conflicts and depended on like any plugin. Its view is a `Plugin` with the
 * package's name and skills and no Claude Code component at all: its `commands/`, `agents/`,
 * `.mcp.json`, `hooks/hooks.json`, `bin/` and `settings.json` are ordinary files (§18.1.1 item 3),
 * no `userConfig` is declared (§18.1.3 item 5) and the manifest is not Claude Code's. Its MCP
 * servers and hook handlers run from the package itself (`coreOf`).
 */

const VIEWS = new WeakMap<CorePackage, Plugin>();
const PACKAGES = new WeakMap<Plugin, CorePackage>();

/** The view of a package, made once per package object so the catalog of its skills is read once. */
function corePluginView(core: CorePackage): Plugin {
  const known = VIEWS.get(core);
  if (known !== undefined) return known;
  const view: Plugin = {
    name: core.name,
    manifest: undefined,
    skills: core.skills,
    commands: [],
    agents: [],
    hooks: {},
    mcpServers: {},
    mcpBundles: {},
    lspServers: {},
    monitors: [],
    outputStyles: [],
    workflows: [],
    themes: [],
    settings: {},
    userConfig: {},
    bin: [],
    hasBin: false,
  };
  VIEWS.set(core, view);
  PACKAGES.set(view, core);
  return view;
}

/** The core package a plugin of the session is the view of, or `undefined` for a Claude Code plugin. */
export const coreOf = (plugin: Plugin): CorePackage | undefined => PACKAGES.get(plugin);

/** A core package the host was given, as the plugin set takes it. */
export const hostPluginOfCore = (entry: HostCorePackage): HostPlugin => ({
  id: entry.id,
  plugin: corePluginView(entry.package),
  ...(entry.enabled === undefined ? {} : { enabled: entry.enabled }),
  ...(entry.source === undefined ? {} : { source: entry.source }),
});
