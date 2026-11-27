import type { InstallScope } from './install-scopes.ts';
import type { ErrorsTabEntry } from './install-messages.ts';
import type { MarketplaceSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What an install says when it stops, and what the Errors tab of `/plugin` shows when a plugin or
 * its marketplace cannot be found, read or trusted (plugins/troubleshooting.md, "Install a plugin",
 * "Plugin installed but not working"; plugins/loading.md, "Check which stage a plugin reached",
 * "Find where a plugin is enabled"). Pure texts; a text the docs quote is quoted verbatim, and the
 * joins the docs do not show are rows in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/install_errors.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/install-errors');

// ---- not found ----

export interface PluginNotFound {
  readonly message: string;
  /** Present when the catalog may be stale or could not be refreshed. */
  readonly hint?: string;
}

/**
 * `Plugin "<name>" not found in marketplace "<marketplace>"`, with a hint when Claude Code did not
 * refresh the marketplace before the lookup: `Your local copy may be out of date — try claude
 * plugin marketplace update <marketplace>`, or `The marketplace couldn't be refreshed (...)` when
 * it tried and failed; with no hint the name is the likeliest problem. The message and the hint
 * are separate values, as the docs print them as two statements.
 */
export const pluginNotFound = (
  plugin: string,
  marketplace: string,
  refresh: { readonly done: boolean; readonly failure?: string },
): PluginNotFound => call('pluginNotFound', plugin, marketplace, refresh);

/** Where an install was typed: `/plugin install` in a session, or `claude plugin install` in a shell. */
export type InstallSurface = 'session' | 'shell';

/** `Plugin "<name>" not found in any marketplace` in a session; `claude plugin install` says `any configured marketplace`. */
export const pluginNotFoundAnywhere = (name: string, surface: InstallSurface): string =>
  call('pluginNotFoundAnywhere', name, surface);

/** The fix for the lookup that found nothing: name the marketplace, so Claude Code refreshes it first. */
export const nameTheMarketplace = (name: string): string => call('nameTheMarketplace', name);

// ---- already installed ----

/** A plugin installed at user scope or by managed settings is "already installed globally". */
export const isInstalledGlobally = (scopes: readonly InstallScope[]): boolean =>
  call('isInstalledGlobally', scopes);

/**
 * `/plugin install` refuses a plugin already installed globally: `Plugin '<name>@<marketplace>' is
 * already installed globally. Use '/plugin' to manage existing plugins.`; typed without
 * `@<marketplace>`, the message omits `globally`.
 */
export const alreadyInstalledGlobally = (plugin: string, withMarketplace: boolean): string =>
  call('alreadyInstalledGlobally', plugin, withMarketplace);

/** What `claude plugin install` prints for a plugin already installed at the target scope (exit 0). */
export const alreadyInstalledAtScope = (id: string, scope: InstallScope): string =>
  call('alreadyInstalledAtScope', id, scope);

// ---- one summary of failures ----

/** A plugin that did not install, and why. */
export interface InstallFailure {
  readonly plugin: string;
  readonly reason: string;
}

/**
 * The summary the `/plugin` menu closes with when none of the selected plugins installed: `Failed
 * to install: <plugin> (<reason>)`. A reason shows only its first line, and when one was
 * shortened the summary ends with `Installing a plugin from its details (Enter) in /plugin shows
 * its full error.` Judgment: several failures are joined by `, `.
 */
export const failedToInstall = (failures: readonly InstallFailure[]): string =>
  call('failedToInstall', failures);

// ---- reload pending ----

/** The notice above the prompt that can appear at the same time. */
export const PLUGINS_CHANGED: 'Plugins changed. Run /reload-plugins to activate.' =
  call('PLUGINS_CHANGED');

export interface InstallActivationFacts {
  /** Activating the plugin now would invalidate the prompt cache. */
  readonly invalidatesPromptCache: boolean;
  /** The attempt to activate the plugin during the install failed. */
  readonly activationFailed: boolean;
}

/**
 * Whether an install activated the plugin, and the line its summary ends with: `Plugin is now
 * active.` when it did, else `Run /reload-plugins to apply.` "Claude Code didn't activate the
 * plugin during the install, either because activating it would invalidate the prompt cache or
 * because the activation attempt failed" (troubleshooting.md).
 */
export const installActivation = (
  facts: InstallActivationFacts,
): {
  readonly activated: boolean;
  readonly ending: string;
} => call('installActivation', facts);

// ---- Errors tab entries ----

/** `Plugin "<name>" not cached at <path>`: the install record points at a directory that is gone. */
export const notCached = (name: string, path: string): ErrorsTabEntry =>
  call('notCached', name, path);

/** The shell command that re-downloads a plugin whose record exists but whose directory is missing. */
export const reinstallCommand = (name: string, marketplace: string): string =>
  call('reinstallCommand', name, marketplace);

/** A project's settings enable the plugin, but it was never installed on this machine. */
export const enabledNotInstalled = (name: string, marketplace: string): ErrorsTabEntry =>
  call('enabledNotInstalled', name, marketplace);

/** An enabled plugin its marketplace lists by relative path, with no directory at that path. */
export const pluginDirectoryNotFound = (path: string): string =>
  call('pluginDirectoryNotFound', path);

/** The marketplace's own directory is missing, such as a local marketplace that moved. */
export const marketplaceDirectoryNotFound = (path: string): string =>
  call('marketplaceDirectoryNotFound', path);

/** A plugin hook's failure when the directory the session loaded its hooks from is gone. */
export const pluginDirectoryMissingAtRun = (path: string, plugin: string): string =>
  call('pluginDirectoryMissingAtRun', path, plugin);

// ---- empty ----

/** Where the empty list shows: the **Discover** tab, or `claude plugin marketplace list`. */
export type EmptyView = 'discover' | 'marketplace-list';

/**
 * `No plugins available` on the Discover tab, `No marketplaces configured` from the list command:
 * both mean no marketplace is registered, so there is no catalog; `undefined` when one is.
 */
export const noMarketplaces = (
  registered: readonly string[],
  view: EmptyView,
): ErrorsTabEntry | undefined => call('noMarketplaces', registered, view);

// ---- a different source ----

/**
 * A source as the "already added from a different source" message writes it: `github:acme/plugins`
 * for GitHub (the docs' example). Judgment (spec/judgments.md): the other kinds are written the
 * same way, `git:<url>`, `npm:<package>`, `settings:<name>`, and a URL or a path as it is.
 */
export const describeSource = (source: MarketplaceSource): string => call('describeSource', source);

/**
 * `Marketplace "<name>" is already added from a different source (<source>). To use this source
 * instead, remove that marketplace first with /plugin marketplace remove <name>.`
 */
export const alreadyAddedFromDifferentSource = (
  name: string,
  existing: MarketplaceSource,
): string => call('alreadyAddedFromDifferentSource', name, existing);

/**
 * "The failure is shown once per session for each hook event and command" (troubleshooting.md,
 * "Plugin directory does not exist"): the returned function says whether this event and command
 * have not failed yet this session, and remembers that they have.
 */
export const failureOnce = (): ((event: string, command: string) => boolean) => call('failureOnce');
