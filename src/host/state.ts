import type { PluginIssue } from '../format/issues.ts';
import type { ResolvedUserConfig } from '../loading/index.ts';
import type { ParentSettingsContext } from '../settings/managed/index.ts';
import { createPolicyView, type PolicyView } from '../settings/index.ts';
import type { CustomizationPolicy } from '../settings/customization/index.ts';
import type { Locations } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import { readCatalog, type PluginCatalog } from './catalog.ts';
import type {
  HostCorePackage,
  HostedPlugin,
  HostPlugin,
  HostUpdate,
  PluginHostOptions,
} from './types.ts';
import { givenOf, hostPluginOfCore, markdownOf } from './core-view.ts';
import type { CoreHandle } from './session-core.ts';
import type { HookSessionGates } from '../settings/hooks/session-gates.ts';

/**
 * What every session of a host shares: the state of one generation of the host, made by the Rust
 * core (`crates/agent-plugins/src/host/state.rs`, docs/RUST.md, "The host session") and held there
 * as one handle a ported module takes back. This module is its edge: what stays the host's (the
 * options with their ports, the `HostPlugin` objects the core names by their place in the list
 * given, the state cells, and `mergeHostOptions` over the host's options).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/state.rs`, docs/RUST.md).
 */

/** A plugin the host runs: what its components read to, and the options saved for it. */
export interface ActivePlugin extends HostPlugin {
  readonly catalog: PluginCatalog;
  readonly userConfig: ResolvedUserConfig;
}

/** What every session of a host shares: the state of one generation of the host; `update` makes the next. */
export interface HostState {
  readonly options: PluginHostOptions;
  readonly view: PolicyView;
  /** How parent settings are treated: the host's, else no administrator and no merge. */
  readonly parent: ParentSettingsContext;
  /** `strictPluginOnlyCustomization` over `view` and `parent`, read once for this generation. */
  readonly customization: CustomizationPolicy;
  /** The hook gates over `view` and `parent`, read once for this generation. */
  readonly gates: HookSessionGates;
  /** Every plugin the host was given, with whether it runs. */
  readonly all: readonly HostedPlugin[];
  /** The plugins that run, in the order given. */
  readonly active: readonly ActivePlugin[];
  readonly issues: readonly PluginIssue[];
  /** The core's state: given back to the core, it is the core's own object again. */
  readonly handle: CoreHandle;
}

const call = coreModule('host/state');

const NO_SETTINGS = createPolicyView([]);
const NO_PARENT: ParentSettingsContext = { hasManagedSettings: false, parentMerge: false };

/** What the core answers: its handle, and each plugin that runs by its place in the list given. */
interface StateAnswer {
  readonly handle: CoreHandle;
  readonly all: readonly HostedPlugin[];
  /** A catalog is given when the core read it in this call: the edge keeps it with the plugin. */
  readonly active: readonly {
    readonly given: number;
    readonly userConfig: ResolvedUserConfig;
    readonly catalog?: PluginCatalog;
  }[];
  readonly issues: readonly PluginIssue[];
  readonly customization: CustomizationPolicy;
  readonly gates: HookSessionGates;
}

/** A plugin as the core takes it: its copy (`givenOf`) in place of the host's object. */
const wireOf = ({ plugin, ...entry }: HostPlugin) => ({ ...entry, plugin: givenOf(plugin).handle });

/** The plugins and the core packages given, as one list in which the core packages come last. */
const givenPlugins = (
  plugins: readonly HostPlugin[] | undefined,
  corePackages: readonly HostCorePackage[] | undefined,
): readonly HostPlugin[] => [...(plugins ?? []), ...(corePackages ?? []).map(hostPluginOfCore)];

/**
 * The state of `options`, with a session's own plugins (`own`) layered over the host's by id in the
 * core. The options' data members cross as `facts`; the ports and `envFiles` stay here.
 */
function stateOf(options: PluginHostOptions, own?: readonly HostPlugin[]): HostState {
  const view = options.settings ?? NO_SETTINGS;
  const parent = options.parentSettings ?? NO_PARENT;
  const plugins = givenPlugins(options.plugins, options.corePackages);
  const given = [...plugins, ...(own ?? [])];
  /** The Markdown of the plugin at `at`, which the core asks for a plugin that runs, once per object. */
  const markdown = (at: number) => {
    const entry = given[at];
    return entry === undefined ? undefined : markdownOf(entry.plugin);
  };
  const made: StateAnswer = call(
    'createHostState',
    {
      plugins: plugins.map(wireOf),
      own: own?.map(wireOf),
      view,
      parent,
      userConfig: options.userConfig,
      facts: {
        windows: options.windows,
        shells: options.shells,
        processId: options.processId,
        hipaa: options.hipaa,
        hookExecution: options.hookExecution,
        shellPaths: options.shellPaths,
        shellArguments: options.shellArguments,
        extraScopeFolders: options.extraScopeFolders,
        launchOwnsEnvironment: options.launchOwnsEnvironment,
        workflows: options.workflows,
      },
    },
    markdown,
  );
  const active = made.active.flatMap(({ given: at, userConfig, catalog }) => {
    const entry = given[at];
    if (entry === undefined) return [];
    if (catalog !== undefined) givenOf(entry.plugin).catalog = catalog;
    return [{ ...entry, catalog: readCatalog(entry.plugin), userConfig }];
  });
  const { handle, all, issues, customization, gates } = made;
  return { options, view, parent, customization, gates, all, active, issues, handle };
}

export const createHostState = (options: PluginHostOptions): HostState => stateOf(options);

/**
 * The options after an update: each part the update gives replaces the host's, a part left out or
 * given as `undefined` stays (as in `applyTurn`: an update replaces, it cannot unset). Pure.
 */
export function mergeHostOptions(base: PluginHostOptions, update: HostUpdate): PluginHostOptions {
  const settings = update.settings ?? base.settings;
  const parentSettings = update.parentSettings ?? base.parentSettings;
  const userConfig = update.userConfig ?? base.userConfig;
  const corePackages = update.corePackages ?? base.corePackages;
  return {
    ...base,
    plugins: update.plugins ?? base.plugins,
    ...(corePackages === undefined ? {} : { corePackages }),
    ...(settings === undefined ? {} : { settings }),
    ...(parentSettings === undefined ? {} : { parentSettings }),
    ...(userConfig === undefined ? {} : { userConfig }),
  };
}

/** What a session reads its state from, at each use. */
export interface HostStateSource {
  current(): HostState;
}

/**
 * The state the host's sessions read, replaced by `host.update`. A session never holds a state
 * past one use: it asks the cell, so an update reaches every session that is open.
 */
export interface HostStateCell extends HostStateSource {
  set(next: HostState): void;
}

/** What a session brings of its own: settings, plugins, saved values and locations (`SessionOptions`). */
export interface SessionOwn {
  readonly settings?: PolicyView | undefined;
  readonly plugins?: readonly HostPlugin[] | undefined;
  readonly corePackages?: readonly HostCorePackage[] | undefined;
  readonly userConfig?: PluginHostOptions['userConfig'] | undefined;
  readonly locations?: Locations | undefined;
}

/**
 * The host's options with what the session brings in place of the host's; its plugins are layered
 * over the host's in the core. The session's locations replace the host's port, so every part of
 * the session that reads `options.ports.locations` (hooks, skills, agents, servers, monitors,
 * folders) reads the session's.
 */
const sessionOptionsOf = (host: PluginHostOptions, own: SessionOwn): PluginHostOptions => {
  const settings = own.settings ?? host.settings;
  const userConfig = own.userConfig ?? host.userConfig;
  return {
    ...host,
    ports: { ...host.ports, locations: own.locations ?? host.ports.locations },
    ...(settings === undefined ? {} : { settings }),
    ...(userConfig === undefined ? {} : { userConfig }),
  };
};

/**
 * The session's own plugins and core packages, or `undefined` when it brings neither. Both forms
 * layer by id together: a session's own core package replaces a host plugin of its id.
 */
const ownPlugins = (own: SessionOwn): readonly HostPlugin[] | undefined =>
  own.plugins === undefined && own.corePackages === undefined
    ? undefined
    : givenPlugins(own.plugins, own.corePackages);

/** Whether the session brings anything of its own, so its state is not the host's. */
const bringsOwn = (own: SessionOwn): boolean =>
  [own.settings, own.plugins, own.corePackages, own.userConfig, own.locations].some(
    (part) => part !== undefined,
  );

/**
 * A session's state: the host's, or, for a session with its own settings, plugins, saved values or
 * locations, the host's options with those in place (`SessionOptions`). Made again only when the
 * host's state changes, so a view that compares states sees one per host generation.
 */
export function sessionStateSource(cell: HostStateSource, own: SessionOwn): HostStateSource {
  if (!bringsOwn(own)) return cell;
  let from: HostState | undefined;
  let made: HostState | undefined;
  return {
    current: () => {
      const host = cell.current();
      if (made === undefined || from !== host) {
        made = stateOf(sessionOptionsOf(host.options, own), ownPlugins(own));
        from = host;
      }
      return made;
    },
  };
}

export function createStateCell(initial: HostState): HostStateCell {
  let state = initial;
  return {
    current: () => state,
    set: (next) => {
      state = next;
    },
  };
}
