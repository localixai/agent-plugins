import { coreModule } from '../wasm/bridge.ts';
import type { ChosenScope } from './install-scopes.ts';

/**
 * Where a plugin is installed from and what each place does (plugins/install.md, "Install a
 * plugin" and its tabs, "Install from your shell", "JetBrains, non-interactive runs, and the Agent
 * SDK"): one table of facts per host, read only through the plans below, which a host runs. The
 * kit opens nothing and adds nothing itself.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/install_hosts.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/install-hosts');

export type InstallHost =
  | 'terminal'
  | 'jetbrains'
  | 'desktop'
  | 'vscode'
  | 'non-interactive'
  | 'agent-sdk'
  | 'cloud';

/** What to use to install and manage plugins on a host. */
export type InstallWith =
  | 'panel'
  | 'terminal-steps'
  | 'plugin-browser'
  | 'manage-plugins'
  | 'shell-commands'
  | 'sdk-plugins-option'
  | 'none';

/** When an install takes effect in a session. */
export type ApplyMoment = 'next-start' | 'reload-plugins' | 'install-summary' | 'open-sessions';

export interface PluginCommandPlan {
  /** What `/plugin` opens: the Discover tab with no plugin name, the plugin's details with one; nothing where the panel does not run. */
  readonly opens: 'discover' | 'details' | undefined;
  /** What Claude replies where `/plugin` does not run, when the docs quote it. */
  readonly reply: string | undefined;
}

/**
 * What `/plugin` does on a host: "run `/plugin` with no plugin name: the panel opens on the
 * Discover tab"; with a name it opens that plugin's details; where it does not run (`claude -p`)
 * Claude replies `/plugin isn't available in this environment.`
 */
export const planPluginCommand = (input: {
  readonly host: InstallHost;
  readonly plugin?: string;
}): PluginCommandPlan => call('planPluginCommand', input);

/** What to use to install and manage plugins on the host. */
export const installWith = (host: InstallHost): InstallWith => call('installWith', host);

/** The scopes the host's install surface offers, in the order the docs list them; none without a surface. */
export const scopeChoices = (host: InstallHost): readonly ChosenScope[] =>
  call('scopeChoices', host);

/** Whether plugins installed on the machine, or enabled by the repository, load on the host; `undefined` where the docs say nothing. */
export const loadsInstalledPlugins = (host: InstallHost): boolean | undefined =>
  call('loadsInstalledPlugins', host);

/** Where an install was made: `claude plugin install` in a shell, or the host's own install surface. */
export type ApplyOrigin = 'shell' | 'surface';

/**
 * When an install takes effect: from a shell, at the next start or on `/reload-plugins`, whatever
 * the host; from the host's own surface, as that surface says (VS Code "apply[ies] to open
 * sessions without a restart"); `undefined` where the docs say nothing.
 */
export const planPluginApply = (
  host: InstallHost,
  origin: ApplyOrigin,
): readonly ApplyMoment[] | undefined => call('planPluginApply', host, origin);

export interface MarketplaceAdd {
  readonly name: string;
  readonly source: string;
}

export const OFFICIAL_MARKETPLACE_ADD: MarketplaceAdd = call('OFFICIAL_MARKETPLACE_ADD');

/**
 * The marketplaces Claude Code adds on its own at start: "Claude Code adds Anthropic's official
 * marketplace for you the first time you start an interactive terminal session". Nothing is added
 * by any other start (a shell command, `claude -p`, another surface) or once it is registered.
 */
export const marketplacesAddedAtStart = (input: {
  readonly host: InstallHost;
  /** No interactive terminal session has run on this machine yet. */
  readonly firstInteractiveStart: boolean;
  readonly registered: readonly string[];
}): readonly MarketplaceAdd[] => call('marketplacesAddedAtStart', input);
