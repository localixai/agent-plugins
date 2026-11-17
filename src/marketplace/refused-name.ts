import { MARKETPLACE_NAME_LOOKS_OFFICIAL, marketplaceNameRefused } from '../format/host-errors.ts';
import { impersonates } from '../format/marketplace/marketplace-names.ts';
import type { MarketplaceSource, MarketplaceSourceKind } from '../format/marketplace/types.ts';

/**
 * `Claude Code refuses the marketplace name` (errors.md): Claude Code "checks the name every time it
 * reads the marketplace's catalog", so a marketplace registered under an imitating name before the
 * check blocked it stops loading, and so do its plugins. The host calls this on every load beside
 * `ignoredReason` and `untrustedSource`. The advice after the quoted prefix is the kit's wording
 * (spec/judgments.md); the steps for a marketplace declared inline in settings are the docs'
 * (v2.1.292 or later).
 */
export interface RefusedMarketplaceName {
  /** The marketplace's own error. */
  readonly marketplace: string;
  /** What `claude plugin list` and the `/plugin` Errors tab show for each of its plugins. */
  readonly plugin: string;
  /** For a marketplace a settings file declares inline (`"source": "settings"`): the steps that keep it. */
  readonly steps?: readonly string[];
}

const DECLARED_IN_SETTINGS: { readonly [K in MarketplaceSourceKind]: boolean } = {
  settings: true,
  github: false,
  git: false,
  url: false,
  npm: false,
  file: false,
  directory: false,
  'skills-dir': false,
  hostPattern: false,
  pathPattern: false,
};

const settingsSteps = (name: string): readonly string[] => [
  "In each settings file that lists the marketplace under extraKnownMarketplaces, give the entry's key and its source.name one new plain-ASCII name that doesn't look like an official one. If an administrator adds the marketplace through your organization's managed settings, the administrator does this step",
  `Run claude plugin marketplace remove ${name}, or remove the old name in the Marketplaces tab in /plugin. This also uninstalls its plugins and deletes their data, options, and secrets`,
  "Start Claude Code again, in the project's folder if the file is a project's settings file. Then install the plugins from the renamed marketplace",
];

/** The refusal of a registered marketplace whose name imitates an official one, or `undefined` when its name loads. */
export function refusedMarketplaceName(
  name: string,
  source: MarketplaceSource,
): RefusedMarketplaceName | undefined {
  if (!impersonates(name)) return undefined;
  const plugin = `${marketplaceNameRefused(name)}: it looks like one of Anthropic's own. Remove it with claude plugin marketplace remove ${name}; this also uninstalls its plugins and deletes their data, options and secrets.`;
  return {
    marketplace: MARKETPLACE_NAME_LOOKS_OFFICIAL,
    plugin,
    ...(DECLARED_IN_SETTINGS[source.kind] ? { steps: settingsSteps(name) } : {}),
  };
}
