import { coreModule } from '../wasm/bridge.ts';

/**
 * Whether a `headersHelper` command may run, and which rules follow from where it was declared
 * (plugins/host-marketplace.md, "Authenticate archive downloads": the table of places; "When Claude
 * Code skips a headersHelper command or drops its output"; "How users accept a headersHelper
 * command"; "Installs and updates that refuse a command instead of asking"; "When a marketplace
 * `url` source's command runs"; settings-reference.md, `extraKnownMarketplaces`,
 * `allowManagedHooksOnly`, `disableCommandPluginSources`). Pure: the host's answers (trust, user
 * acceptance, policy) come in as data; the acceptance prompt itself is the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/header_policy.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/header-policy');

/** The two places `headers` and `headersHelper` can be set. */
export const HEADER_PLACES: readonly ['url-source', 'entry'] = call('HEADER_PLACES');
export type HeaderPlace = (typeof HEADER_PLACES)[number];

/**
 * Where the declaration lives: a `marketplace.json` entry (`catalog`), or a settings file: user
 * settings, a `--settings` file, a managed settings file on the machine, server-managed settings,
 * a project's `.claude/settings.json` or `settings.local.json`, or an `--add-dir` directory's.
 */
export const HEADER_ORIGINS: readonly [
  'catalog',
  'user',
  'cli-settings',
  'managed',
  'server-managed',
  'project',
  'add-dir',
] = call('HEADER_ORIGINS');
export type HeaderOrigin = (typeof HEADER_ORIGINS)[number];

/** What the user is doing when an entry's archive is about to be downloaded. */
export const PLUGIN_OPERATIONS: readonly [
  'install',
  'update',
  'bulk-install',
  'suggestion',
  'dependency',
  'background-update',
  'session-start',
] = call('PLUGIN_OPERATIONS');
export type PluginOperation = (typeof PLUGIN_OPERATIONS)[number];

/**
 * The command and archive URL the user was shown and accepted. `queryCounts` is set for the VS Code
 * extension and `--accept-command`, where a change in the query string is a change too.
 */
export interface AcceptedCommand {
  readonly command: string;
  readonly archiveUrl: string;
  readonly queryCounts?: boolean;
}

/** One `headers`/`headersHelper` declaration, with what the policy needs to know about it. */
export interface HeaderLayer {
  readonly place: HeaderPlace;
  readonly origin: HeaderOrigin;
  readonly headers?: Readonly<Record<string, string>>;
  readonly headersHelper?: string;
  /** The `CLAUDE_CODE_*` variables Claude Code sets for the command (`urlSourceLayer`, `entryLayer`). */
  readonly variables: Readonly<Record<string, string>>;
  /** A `url` source: the marketplace URL the command is declared for. */
  readonly marketplaceUrl?: string;
  /** An entry: the archive URL its command runs for. */
  readonly archiveUrl?: string;
  /** An entry: what the user is doing, and the command and archive URL they accepted. */
  readonly operation?: PluginOperation;
  readonly accepted?: AcceptedCommand;
}

/** What the host knows when it asks. */
export interface HeadersSetting {
  readonly platform: 'posix' | 'windows';
  /** The user accepted the workspace trust dialog for the project folder itself. */
  readonly folderTrusted: boolean;
  /** The user approved the delivered server-managed settings in the security approval dialog. */
  readonly settingsApproved: boolean;
  readonly disableCommandPluginSources?: boolean;
  readonly allowManagedHooksOnly?: boolean;
  /** Managed settings themselves declare the marketplace the command belongs to. */
  readonly managedMarketplace: boolean;
}

/** What the host knows when it asks. */
export interface HeadersSetting {
  readonly platform: 'posix' | 'windows';
  /** The user accepted the workspace trust dialog for the project folder itself. */
  readonly folderTrusted: boolean;
  /** The user approved the delivered server-managed settings in the security approval dialog. */
  readonly settingsApproved: boolean;
  readonly disableCommandPluginSources?: boolean;
  readonly allowManagedHooksOnly?: boolean;
  /** Managed settings themselves declare the marketplace the command belongs to. */
  readonly managedMarketplace: boolean;
}

/**
 * Why a command did not run. `skip-helper`: the fixed `headers` still go (judgment where the docs
 * only say the command does not run). `refuse-download`: the plugin is not downloaded at all.
 */
export const HELPER_REASONS: readonly [
  'ignored-add-dir',
  'not-https',
  'folder-not-trusted',
  'settings-not-approved',
  'blocked-by-policy',
  'not-accepted',
  'needs-own-view',
  'listed-in-errors',
] = call('HELPER_REASONS');
export type HelperReason = (typeof HELPER_REASONS)[number];

export type HelperEffect = 'skip-helper' | 'refuse-download';

export type HelperVerdict =
  | { readonly allowed: true }
  | {
      readonly allowed: false;
      readonly reason: HelperReason;
      readonly effect: HelperEffect;
      readonly message: string;
    };

/** Whether the layer's command may run now, or why not (and what that does to the download). */
export const helperAllowed = (layer: HeaderLayer, setting: HeadersSetting): HelperVerdict =>
  call('helperAllowed', layer, setting);
