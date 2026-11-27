import type { CorePackage } from '../loading/core/index.ts';
import type { Plugin } from '../loading/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginCatalog } from './catalog.ts';
import type { CoreHandle } from './session-core.ts';
import type { HostCorePackage, HostPlugin } from './types.ts';

/**
 * A core package as the session's plugin machinery sees it (Agent Plugins Extended §18.1.5, J18.15):
 * a plugin to every part of the session that is not a component, known by its id, enabled,
 * restricted, named in conflicts and depended on like any plugin. Its view is a `Plugin` with the
 * package's name and skills and no Claude Code component at all: its `commands/`, `agents/`,
 * `.mcp.json`, `hooks/hooks.json`, `bin/` and `settings.json` are ordinary files (§18.1.1 item 3),
 * no `userConfig` is declared (§18.1.3 item 5) and the manifest is not Claude Code's. Its MCP
 * servers and hook handlers run from the package itself (`coreOf`).
 *
 * Each `Plugin` and `CorePackage` object the host gives crosses to the core once: this edge keeps
 * the core's copy of it by the object's identity (as an engine numbers its handlers, docs/RUST.md,
 * "The host session"), and a package's view, which the core makes, by both objects. A plugin's copy
 * leaves out its Markdown, which crosses only when its catalog is read (`markdownOf`), so a plugin
 * that never runs has its Markdown neither read nor copied.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/core_view.rs`, docs/RUST.md).
 */

const call = coreModule('host/core-view');

/** The core's copy of one plugin the host gave, and what the edge keeps beside it. */
export interface Given {
  /** The core's copy (`GivenPlugin`). */
  readonly handle: CoreHandle;
  /** The plugin, or a core package's view. */
  readonly plugin: Plugin;
  /** The core package the view is of. */
  readonly core?: CorePackage;
  /** Its catalog once read (`readCatalog`). */
  catalog?: PluginCatalog;
}

const GIVEN = new WeakMap<object, Given>();

/** The members of a `Plugin` its catalog reads, copied only then. */
const MARKDOWN: ReadonlySet<string> = new Set(['skills', 'commands', 'agents', 'outputStyles']);

/** The plugin without its Markdown: only the other members are read. */
const headOf = (plugin: Plugin): Readonly<Record<string, unknown>> =>
  Object.fromEntries(
    Object.keys(plugin)
      .filter((key) => !MARKDOWN.has(key))
      .map((key) => [key, Reflect.get(plugin, key)]),
  );

/** The Markdown of a plugin, as its catalog reads it. */
export const markdownOf = (
  plugin: Plugin,
): Pick<Plugin, 'skills' | 'commands' | 'agents' | 'outputStyles'> => ({
  skills: plugin.skills,
  commands: plugin.commands,
  agents: plugin.agents,
  outputStyles: plugin.outputStyles,
});

/** The core's copy of a plugin the host gave, made at its first use. */
export function givenOf(plugin: Plugin): Given {
  const known = GIVEN.get(plugin);
  if (known !== undefined) return known;
  const given: Given = { handle: call('pluginData', headOf(plugin)), plugin };
  GIVEN.set(plugin, given);
  return given;
}

/** The core's copy of a core package and its view, made once per package object. */
function givenOfCore(core: CorePackage): Given {
  const known = GIVEN.get(core);
  if (known !== undefined) return known;
  const made: { readonly given: CoreHandle; readonly view: Plugin } = call('corePackageData', core);
  const given: Given = { handle: made.given, plugin: made.view, core };
  GIVEN.set(core, given);
  GIVEN.set(made.view, given);
  return given;
}

/** The core package a plugin of the session is the view of, or `undefined` for a Claude Code plugin. */
export const coreOf = (plugin: Plugin): CorePackage | undefined => GIVEN.get(plugin)?.core;

/** A core package the host was given, as the plugin set takes it. */
export const hostPluginOfCore = (entry: HostCorePackage): HostPlugin => ({
  id: entry.id,
  plugin: givenOfCore(entry.package).plugin,
  ...(entry.enabled === undefined ? {} : { enabled: entry.enabled }),
  ...(entry.source === undefined ? {} : { source: entry.source }),
});
