/**
 * `extraKnownMarketplaces` as the plan a host executes: which marketplaces register, from what, and
 * why a declaration does not (settings-reference.md, `extraKnownMarketplaces`, "Marketplace source
 * types", "Marketplace key aliases"; plugins/org.md, "Require a marketplace and its plugins",
 * "Require plugins per repository", "Allow the official marketplace and your own"; plugins/loading.md,
 * "Which marketplaces and plugins auto-update"). Cloning, refreshing and running a `headersHelper`
 * are the host's. The aliases are already rewritten by `readSettingsLayer`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/registry.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { SdkMarketplaceSource } from '../../protocol/official.ts';
import type { KeyHolder, PolicyScope } from '../layers.ts';
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/marketplaces/registry');

/**
 * Anthropic's official marketplace, which "needs no `extraKnownMarketplaces` entry when
 * `enabledPlugins` sets one of its plugins to `true`" (plugins/org.md). Its source is the one the
 * docs' allowlist example names ("Allow only the official marketplace").
 */
export const OFFICIAL_MARKETPLACE: 'claude-plugins-official' = call('OFFICIAL_MARKETPLACE');

/**
 * - `register`: the host adds the marketplace from this entry.
 * - `ignoredUntrusted`: a project, local or `--add-dir` entry in a folder that is not trusted.
 * - `refusedByAllowlist`: the allowlist or blocklist does not let the source in; `issues` says why.
 * - `invalid`: the entry cannot name a marketplace (`issues` says why).
 * - `shadowed`: a higher-precedence file declares the same name; this entry is replaced whole.
 */
export type RegistrationStatus =
  | 'register'
  | 'ignoredUntrusted'
  | 'refusedByAllowlist'
  | 'invalid'
  | 'shadowed';

export interface Registration {
  readonly name: string;
  readonly entry: { readonly source: SdkMarketplaceSource; readonly autoUpdate: boolean };
  readonly from: PolicyScope;
  readonly status: RegistrationStatus;
  /** The official marketplace declared by an `enabledPlugins` entry, not by an `extraKnownMarketplaces` one. */
  readonly implied: boolean;
  /** A registered managed entry that sets `autoUpdate`: the user's `/plugin` toggle is refused. */
  readonly autoUpdateFixed: boolean;
  /**
   * The settings file whose entry sets `autoUpdate`, by its scope and its position among the layers;
   * absent when this entry leaves it unset and `entry.autoUpdate` is the default (a lower entry's
   * value never carries over: "inherits none of its fields"). Judgment (spec/judgments.md).
   */
  readonly autoUpdateFrom?: SettingsFile;
  readonly issues: readonly PluginIssue[];
}

/** One settings file, as the layers order them: two files of one scope have two positions. */
export type SettingsFile = Pick<KeyHolder<'extraKnownMarketplaces'>, 'scope' | 'position'>;

/** What the registry needs from the host, as plain data and one injected check. */
export interface RegistryContext {
  /** Whether the user accepted the workspace trust dialog for the folder of that kind of file. */
  readonly isFolderTrusted: (scope: 'project' | 'local' | 'addDir') => boolean;
  /** Marketplaces Anthropic runs whose `autoUpdate` defaults on; Anthropic's list is not in the docs. */
  readonly officialMarketplaces: readonly string[];
  /** The marketplace gate's `check` (allowlist and blocklist), taken as a function. */
  readonly check: (
    source: SdkMarketplaceSource,
  ) => { readonly allowed: true } | { readonly allowed: false; readonly message: string };
}

export interface MarketplaceRegistry {
  /** Every declaration, per name from the highest-precedence file down; names in order of appearance. */
  registrations(): readonly Registration[];
}

/** The registry over a view; the context's functions are asked each time `registrations` runs. */
export const createMarketplaceRegistry = (
  view: PolicyView,
  context: RegistryContext,
): MarketplaceRegistry => call('createMarketplaceRegistry', view, context);
